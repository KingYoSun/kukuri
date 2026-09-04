#![cfg(target_os = "linux")]

use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn cli_command(root: &Path, profile: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kukuri-cli"));
    command
        .args(["--profile", profile])
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_RUNTIME_DIR", root.join("runtime"))
        .env("HOME", root.join("home"))
        .env("KUKURI_DISABLE_KEYRING", "1")
        .env_remove("KUKURI_APP_DATA_DIR")
        .env_remove("KUKURI_INSTANCE");
    command
}

fn daemon_command(root: &Path, profile: &str) -> Command {
    let mut command = cli_command(root, profile);
    command.args(["daemon", "run"]);
    command
}

fn accept_consent(root: &Path, profile: &str) {
    let status = cli_command(root, profile)
        .args(["consent", "accept", "--accept-documents", "--age-confirmed"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .expect("accept consent");
    assert!(status.success(), "consent command exited with {status}");
}

fn spawn_daemon(root: &Path, profile: &str) -> Child {
    daemon_command(root, profile)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn daemon")
}

fn socket_path(root: &Path, profile: &str) -> PathBuf {
    root.join("runtime")
        .join("kukuri")
        .join(format!("{profile}.sock"))
}

fn wait_for_ready(child: &mut Child, path: &Path) {
    let stdout = child.stdout.take().expect("daemon stdout");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let result = reader.read_line(&mut line).map(|_| line);
        let _ = sender.send(result);
    });
    let line = match receiver.recv_timeout(Duration::from_secs(20)) {
        Ok(result) => result.expect("read daemon readiness"),
        Err(error) => {
            let _ = child.kill();
            panic!("daemon did not report readiness: {error}");
        }
    };
    assert_eq!(line.trim(), "ready", "unexpected daemon readiness");
    assert!(
        path.exists(),
        "daemon socket does not exist: {}",
        path.display()
    );
    UnixStream::connect(path).expect("connect daemon socket");
}

fn terminate(mut child: Child) {
    let result = unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    assert_eq!(result, 0, "send SIGTERM");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("wait daemon") {
            assert!(status.success(), "daemon exited with {status}");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "daemon did not stop after SIGTERM"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn process_tcp_listener_count(pid: u32) -> usize {
    let socket_inodes = fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("process fds")
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .filter_map(|target| {
            let value = target.to_string_lossy();
            value
                .strip_prefix("socket:[")
                .and_then(|value| value.strip_suffix(']'))
                .map(str::to_string)
        })
        .collect::<std::collections::HashSet<_>>();

    ["/proc/net/tcp", "/proc/net/tcp6"]
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .flat_map(|table| {
            table
                .lines()
                .skip(1)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|line| {
            let columns = line.split_ascii_whitespace().collect::<Vec<_>>();
            columns.get(3) == Some(&"0A")
                && columns
                    .get(9)
                    .is_some_and(|inode| socket_inodes.contains(*inode))
        })
        .count()
}

#[test]
fn daemon_enforces_profile_ownership_permissions_and_graceful_restart() {
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("runtime")).expect("runtime root");
    fs::create_dir_all(root.path().join("home")).expect("home");
    accept_consent(root.path(), "alpha");
    accept_consent(root.path(), "beta");

    let alpha_socket = socket_path(root.path(), "alpha");
    let beta_socket = socket_path(root.path(), "beta");
    let mut alpha = spawn_daemon(root.path(), "alpha");
    wait_for_ready(&mut alpha, &alpha_socket);

    assert_eq!(
        fs::metadata(root.path().join("runtime/kukuri"))
            .expect("runtime metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&alpha_socket)
            .expect("socket metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(process_tcp_listener_count(alpha.id()), 0);

    let duplicate = daemon_command(root.path(), "alpha")
        .output()
        .expect("duplicate daemon");
    assert!(!duplicate.status.success());
    assert!(
        String::from_utf8_lossy(&duplicate.stderr).contains("profile_in_use"),
        "{}",
        String::from_utf8_lossy(&duplicate.stderr)
    );

    let mut beta = spawn_daemon(root.path(), "beta");
    wait_for_ready(&mut beta, &beta_socket);
    terminate(alpha);
    terminate(beta);
    assert!(!alpha_socket.exists());
    assert!(!beta_socket.exists());

    let mut restarted = spawn_daemon(root.path(), "alpha");
    wait_for_ready(&mut restarted, &alpha_socket);
    terminate(restarted);
    assert!(!alpha_socket.exists());
}
