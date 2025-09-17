use std::fs;
use std::path::PathBuf;
use std::process::Command;

use base64::prelude::*;
use iroh::SecretKey;

fn docker_command() -> Command {
    Command::new("docker")
}

fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

struct TempEnvFile {
    path: PathBuf,
}

impl TempEnvFile {
    fn write(path: PathBuf, contents: String) -> Self {
        fs::write(&path, contents).unwrap_or_else(|err| {
            panic!(
                "failed to write temporary env file {}: {}",
                path.display(),
                err
            )
        });
        Self { path }
    }
}

impl Drop for TempEnvFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn ensure_docker_available() -> bool {
    match docker_command().arg("--version").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn base64_secret(secret: &[u8; 32]) -> String {
    BASE64_STANDARD.encode(secret)
}

fn node_id_for(secret: &[u8; 32]) -> String {
    SecretKey::from_bytes(secret).public().to_string()
}

#[test]
#[ignore]
fn docker_connectivity_probe() {
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        eprintln!("SKIP_DOCKER_TESTS=1 set, skipping docker connectivity probe");
        return;
    }

    if !ensure_docker_available() {
        eprintln!("docker command not found, skipping docker connectivity probe");
        return;
    }

    let secret_a = [0x11u8; 32];
    let secret_b = [0x22u8; 32];

    let node_a_secret_b64 = base64_secret(&secret_a);
    let node_b_secret_b64 = base64_secret(&secret_b);
    let node_a_id = node_id_for(&secret_a);
    println!("node_a_id={}", node_a_id);
    let node_a_addr = format!("{}@node_a:11223", node_a_id);

    let tests_dir = tests_dir();
    fs::create_dir_all(&tests_dir).expect("failed to prepare tests directory");
    let _env_node_a = TempEnvFile::write(
        tests_dir.join(".env.node_a"),
        format!("KUKURI_SECRET_KEY={}\n", node_a_secret_b64),
    );
    let _env_node_b = TempEnvFile::write(
        tests_dir.join(".env.node_b"),
        format!(
            "KUKURI_SECRET_KEY={}\nNODE_A_ADDR={}\nCONNECT_PEER={}\n",
            node_b_secret_b64, node_a_addr, node_a_addr
        ),
    );

    let mut up_cmd = docker_command();
    up_cmd
        .args([
            "compose",
            "-f",
            "docker-compose.test.yml",
            "up",
            "--abort-on-container-exit",
            "--build",
        ])
        .current_dir(&tests_dir)
        .env("COMPOSE_PROJECT_NAME", "kukuri_cli_docker_test");

    let output = up_cmd.output().expect("failed to invoke docker compose up");

    let mut down_cmd = docker_command();
    let _ = down_cmd
        .args([
            "compose",
            "-f",
            "docker-compose.test.yml",
            "down",
            "--volumes",
            "--remove-orphans",
        ])
        .current_dir(&tests_dir)
        .env("COMPOSE_PROJECT_NAME", "kukuri_cli_docker_test")
        .output();

    if !output.status.success() {
        panic!(
            "docker compose up failed: status {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
