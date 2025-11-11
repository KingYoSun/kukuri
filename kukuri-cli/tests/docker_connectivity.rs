use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Clone, Copy)]
enum ConnectivityScenario {
    DirectNoDht,
    MdnsDiscovery,
}

impl ConnectivityScenario {
    fn slug(&self) -> &'static str {
        match self {
            ConnectivityScenario::DirectNoDht => "direct_no_dht",
            ConnectivityScenario::MdnsDiscovery => "mdns_discovery",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ConnectivityScenario::DirectNoDht => "direct connection without DHT",
            ConnectivityScenario::MdnsDiscovery => "connect via mdns/discovery",
        }
    }

    fn connect_args(&self) -> &'static str {
        match self {
            ConnectivityScenario::DirectNoDht => "--no-dht --timeout 30",
            ConnectivityScenario::MdnsDiscovery => "--timeout 45 --mdns",
        }
    }
}

fn should_skip_docker() -> Option<&'static str> {
    if std::env::var("SKIP_DOCKER_TESTS").is_ok() {
        return Some("SKIP_DOCKER_TESTS=1");
    }
    if !ensure_docker_available() {
        return Some("docker command not found");
    }
    None
}

fn write_env_files(dir: &Path, scenario: ConnectivityScenario) -> (TempEnvFile, TempEnvFile) {
    let secret_a = [0x11u8; 32];
    let secret_b = [0x22u8; 32];

    let node_a_secret_b64 = base64_secret(&secret_a);
    let node_b_secret_b64 = base64_secret(&secret_b);
    let node_a_id = node_id_for(&secret_a);
    let node_a_addr = format!("{}@node_a:11223", node_a_id);

    let node_a_env = TempEnvFile::write(
        dir.join(".env.node_a"),
        format!("KUKURI_SECRET_KEY={}\n", node_a_secret_b64),
    );

    let node_b_env = TempEnvFile::write(
        dir.join(".env.node_b"),
        format!(
            "KUKURI_SECRET_KEY={}\nNODE_A_ADDR={}\nCONNECT_PEER={}\nCONNECT_ARGS=\"{}\"\n",
            node_b_secret_b64,
            node_a_addr,
            node_a_addr,
            scenario.connect_args()
        ),
    );

    (node_a_env, node_b_env)
}

fn run_docker_connectivity_scenario(scenario: ConnectivityScenario) {
    if let Some(reason) = should_skip_docker() {
        eprintln!(
            "Skipping docker connectivity test ({}) due to: {}",
            scenario.description(),
            reason
        );
        return;
    }

    let tests_dir = tests_dir();
    fs::create_dir_all(&tests_dir).expect("failed to prepare tests directory");
    let _env_files = write_env_files(&tests_dir, scenario);
    let project_name = format!("kukuri_cli_{}", scenario.slug());

    let mut up_cmd = docker_command();
    up_cmd
        .args([
            "compose",
            "-f",
            "docker-compose.test.yml",
            "up",
            "--abort-on-container-exit",
            "--build",
            "node_b",
        ])
        .current_dir(&tests_dir)
        .env("COMPOSE_PROJECT_NAME", &project_name);

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
        .env("COMPOSE_PROJECT_NAME", &project_name)
        .output();

    assert!(
        output.status.success(),
        "docker compose up failed for {}: status {:?}\nstdout:\n{}\nstderr:\n{}",
        scenario.description(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn docker_connectivity_direct_no_dht() {
    run_docker_connectivity_scenario(ConnectivityScenario::DirectNoDht);
}

#[test]
fn docker_connectivity_with_mdns() {
    run_docker_connectivity_scenario(ConnectivityScenario::MdnsDiscovery);
}
