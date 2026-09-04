use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use kukuri_desktop_runtime::{ClientHost, ClientHostStart, ClientProfile, ProfileLease};
use tokio::{
    io::AsyncReadExt,
    net::{UnixListener, UnixStream},
    process::Command,
};

use crate::{CliError, DaemonCommand};

pub(crate) async fn run(profile: ClientProfile, command: DaemonCommand) -> Result<(), CliError> {
    match command {
        DaemonCommand::Run => run_foreground(profile).await,
        DaemonCommand::Start => systemctl(&profile.name, "start").await,
        DaemonCommand::Stop => systemctl(&profile.name, "stop").await,
        DaemonCommand::Status => systemctl(&profile.name, "status").await,
    }
}

async fn run_foreground(profile: ClientProfile) -> Result<(), CliError> {
    let lease = ProfileLease::acquire(profile).map_err(CliError::from_profile)?;
    let socket_path = daemon_socket_path(&lease)?;
    let listener = bind_listener(&socket_path)?;
    let _socket_cleanup = SocketCleanup(socket_path);

    let host = match ClientHost::start_if_consented(lease.profile().app_data_dir.clone())
        .await
        .map_err(|error| CliError::new("host_start_failed", error.to_string(), 1))?
    {
        ClientHostStart::Ready(host) => Some(host),
        ClientHostStart::ConsentRequired(_) => None,
    };
    println!(
        "{}",
        if host.is_some() {
            "ready"
        } else {
            "consent_required"
        }
    );

    wait_for_shutdown(&listener).await?;
    if let Some(host) = host {
        host.shutdown().await;
    }
    drop(lease);
    Ok(())
}

async fn wait_for_shutdown(listener: &UnixListener) -> Result<(), CliError> {
    let terminate = async {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(|error| CliError::new("signal_setup_failed", error.to_string(), 1))?;
        terminate.recv().await;
        Ok::<(), CliError>(())
    };
    tokio::pin!(terminate);

    loop {
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                result.map_err(|error| CliError::new("signal_wait_failed", error.to_string(), 1))?;
                return Ok(());
            }
            result = &mut terminate => return result,
            result = listener.accept() => {
                let (stream, _) = result.map_err(|error| {
                    CliError::new("socket_accept_failed", error.to_string(), 1)
                })?;
                match authorize_peer(&stream) {
                    Ok(()) => {
                        tokio::spawn(drain_connection(stream));
                    }
                    Err(error) => eprintln!("{}: {}", error.code, error.message),
                }
            }
        }
    }
}

async fn drain_connection(mut stream: UnixStream) {
    let mut buffer = [0_u8; 1024];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

fn daemon_socket_path(lease: &ProfileLease) -> Result<PathBuf, CliError> {
    use std::os::unix::ffi::OsStrExt;

    let runtime_root = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            CliError::new(
                "runtime_dir_unavailable",
                "XDG_RUNTIME_DIR is required for the daemon socket",
                1,
            )
        })?;
    let directory = runtime_root.join("kukuri");
    ensure_private_runtime_directory(&directory)?;
    let profile_path = std::fs::canonicalize(&lease.profile().app_data_dir).map_err(|error| {
        CliError::new(
            "profile_path_unavailable",
            format!(
                "failed to resolve profile directory `{}`: {error}",
                lease.profile().app_data_dir.display()
            ),
            1,
        )
    })?;
    let digest = blake3::hash(profile_path.as_os_str().as_bytes()).to_hex();
    Ok(directory.join(format!("profile-{}.sock", &digest[..32])))
}

fn ensure_private_runtime_directory(path: &Path) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && metadata.file_type().is_symlink()
    {
        return Err(CliError::new(
            "runtime_dir_invalid",
            format!(
                "runtime directory `{}` must not be a symlink",
                path.display()
            ),
            1,
        ));
    }
    std::fs::create_dir_all(path).map_err(|error| {
        CliError::new(
            "runtime_dir_unavailable",
            format!("failed to create `{}`: {error}", path.display()),
            1,
        )
    })?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(|error| {
        CliError::new(
            "runtime_dir_permission_failed",
            format!("failed to protect `{}`: {error}", path.display()),
            1,
        )
    })
}

fn bind_listener(path: &Path) -> Result<UnixListener, CliError> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};

    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if !metadata.file_type().is_socket() {
            return Err(CliError::new(
                "socket_path_invalid",
                format!("socket path `{}` is not a socket", path.display()),
                1,
            ));
        }
        std::fs::remove_file(path).map_err(|error| {
            CliError::new(
                "socket_cleanup_failed",
                format!(
                    "failed to remove stale socket `{}`: {error}",
                    path.display()
                ),
                1,
            )
        })?;
    }
    let listener = UnixListener::bind(path).map_err(|error| {
        CliError::new(
            "socket_bind_failed",
            format!("failed to bind `{}`: {error}", path.display()),
            1,
        )
    })?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|error| {
        CliError::new(
            "socket_permission_failed",
            format!("failed to protect `{}`: {error}", path.display()),
            1,
        )
    })?;
    Ok(listener)
}

fn authorize_peer(stream: &UnixStream) -> Result<(), CliError> {
    let credential = stream
        .peer_cred()
        .map_err(|error| CliError::new("peer_credential_failed", error.to_string(), 1))?;
    authorize_peer_uid(unsafe { libc::geteuid() }, credential.uid())
}

fn authorize_peer_uid(expected_uid: u32, actual_uid: u32) -> Result<(), CliError> {
    if expected_uid == actual_uid {
        Ok(())
    } else {
        Err(CliError::new(
            "peer_uid_mismatch",
            format!("peer uid {actual_uid} does not match daemon uid {expected_uid}"),
            1,
        ))
    }
}

async fn systemctl(profile: &str, verb: &str) -> Result<(), CliError> {
    let unit = format!("kukuri@{profile}.service");
    let output = Command::new("systemctl")
        .args(["--user", verb, unit.as_str()])
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| CliError::new("systemd_unavailable", error.to_string(), 1))?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if output.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(CliError::new(
            "systemd_command_failed",
            if message.is_empty() {
                format!("systemctl --user {verb} {unit} failed")
            } else {
                message
            },
            output.status.code().unwrap_or(1),
        ))
    }
}

struct SocketCleanup(PathBuf);

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use kukuri_desktop_runtime::{ClientProfileKind, resolve_cli_profile};

    use super::*;

    #[tokio::test]
    async fn runtime_directory_and_socket_are_private() {
        let root = tempfile::tempdir().expect("tempdir");
        let runtime_dir = root.path().join("runtime");
        ensure_private_runtime_directory(&runtime_dir).expect("runtime dir");
        let socket = runtime_dir.join("test.sock");
        let listener = bind_listener(&socket).expect("listener");
        assert_eq!(
            std::fs::metadata(&runtime_dir)
                .expect("dir metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&socket)
                .expect("socket metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        drop(listener);
    }

    #[test]
    fn peer_uid_mismatch_is_rejected() {
        let error = authorize_peer_uid(1000, 1001).expect_err("different uid");
        assert_eq!(error.code, "peer_uid_mismatch");
    }

    #[test]
    fn profile_kind_for_daemon_is_cli() {
        let profile =
            resolve_cli_profile(Some("alpha"), None, None, Some(Path::new("/data")), None)
                .expect("profile");
        assert_eq!(profile.kind, ClientProfileKind::Cli);
    }
}
