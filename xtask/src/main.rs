use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        bail!("missing xtask command");
    };

    match command.as_str() {
        "doctor" => doctor(),
        "check" => check(),
        "test" => test(),
        "cn-check" => cn_check(),
        "cn-test" => cn_test(),
        "desktop-package" => desktop_package(),
        "e2e-smoke" => e2e_smoke("desktop_smoke_post_persist"),
        "scenario" => {
            let name = args.next().context("scenario name is required")?;
            scenario(name.as_str())
        }
        _ => {
            print_usage();
            bail!("unsupported xtask command: {command}");
        }
    }
}

fn root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("workspace root")
        .to_path_buf()
}

fn desktop_dir() -> PathBuf {
    root_dir().join("apps").join("desktop")
}

fn artifacts_dir(name: &str) -> PathBuf {
    root_dir()
        .join("test-results")
        .join("kukuri")
        .join(name.replace('/', "-"))
}

fn doctor() -> Result<()> {
    for binary in ["cargo", "rustc", "node"] {
        run_capture(binary, ["--version"], &root_dir())
            .with_context(|| format!("required dependency is missing: {binary}"))?;
    }
    run_pnpm(["--version"], &desktop_dir()).context("required dependency is missing: pnpm")?;

    for required_path in [
        root_dir().join("Cargo.toml"),
        desktop_dir().join("package.json"),
        root_dir()
            .join("harness")
            .join("scenarios")
            .join("desktop_smoke_post_persist.yaml"),
    ] {
        if !required_path.exists() {
            bail!("required path is missing: {}", required_path.display());
        }
    }

    Ok(())
}

fn check() -> Result<()> {
    run("cargo", ["fmt", "--check"], &root_dir())?;
    run(
        "cargo",
        [
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        &root_dir(),
    )?;
    run(
        "cargo",
        [
            "check",
            "--manifest-path",
            "apps/desktop/src-tauri/Cargo.toml",
        ],
        &root_dir(),
    )?;
    run("cargo", ["test", "--workspace", "--lib"], &root_dir())?;
    run_pnpm(["lint"], &desktop_dir())?;
    run_pnpm(["typecheck"], &desktop_dir())?;
    run_pnpm(["test"], &desktop_dir())?;
    Ok(())
}

fn test() -> Result<()> {
    run("cargo", ["test", "--workspace"], &root_dir())?;
    run_pnpm(["test"], &desktop_dir())?;
    Ok(())
}

fn cn_check() -> Result<()> {
    run(
        "cargo",
        [
            "check",
            "-p",
            "kukuri-cn-core",
            "-p",
            "kukuri-cn-user-api",
            "-p",
            "kukuri-cn-iroh-relay",
            "-p",
            "kukuri-cn-cli",
        ],
        &root_dir(),
    )
}

fn cn_test() -> Result<()> {
    with_cn_postgres(|| {
        run_with_env(
            "cargo",
            [
                "test",
                "-p",
                "kukuri-cn-core",
                "-p",
                "kukuri-cn-user-api",
                "-p",
                "kukuri-cn-iroh-relay",
                "-p",
                "kukuri-cn-cli",
            ],
            &root_dir(),
            &cn_test_envs(),
        )
    })
}

fn e2e_smoke(name: &str) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to build tokio runtime")?;
    let result = runtime.block_on(kukuri_harness::run_named_scenario(
        &root_dir(),
        name,
        &artifacts_dir(name),
    ))?;
    let metrics = kukuri_harness::summarize_metrics(&result);
    for (key, value) in metrics {
        println!("{key}={value}");
    }
    Ok(())
}

fn scenario(name: &str) -> Result<()> {
    if scenario_requires_cn_postgres(name) {
        with_cn_postgres(|| e2e_smoke(name))
    } else {
        e2e_smoke(name)
    }
}

fn desktop_package() -> Result<()> {
    if !cfg!(target_os = "windows") {
        bail!("desktop-package is only supported on Windows hosts");
    }

    run_pnpm(
        ["tauri", "build", "--target", "x86_64-pc-windows-msvc"],
        &desktop_dir(),
    )
}

fn run(binary: &str, args: impl IntoIterator<Item = &'static str>, cwd: &Path) -> Result<()> {
    run_with_env(binary, args, cwd, &[])
}

fn run_with_env(
    binary: &str,
    args: impl IntoIterator<Item = &'static str>,
    cwd: &Path,
    envs: &[(&str, &str)],
) -> Result<()> {
    let status = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .envs(envs.iter().copied())
        .status()
        .with_context(|| format!("failed to execute {binary}"))?;
    if !status.success() {
        bail!("{binary} exited with status {status}");
    }
    Ok(())
}

fn run_capture(
    binary: &str,
    args: impl IntoIterator<Item = &'static str>,
    cwd: &Path,
) -> Result<()> {
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to execute {binary}"))?;
    if !output.status.success() {
        bail!("{binary} exited with status {}", output.status);
    }
    Ok(())
}

fn run_pnpm(args: impl IntoIterator<Item = &'static str>, cwd: &Path) -> Result<()> {
    let args = args.into_iter().collect::<Vec<_>>();
    if Command::new("pnpm")
        .arg("--version")
        .current_dir(cwd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
    {
        run("pnpm", args.iter().copied(), cwd)
    } else {
        let mut fallback = vec!["--yes", "pnpm@10.16.1"];
        fallback.extend(args);
        run("npx", fallback, cwd)
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo xtask <doctor|check|test|cn-check|cn-test|desktop-package|e2e-smoke|scenario <name>>"
    );
}

fn cn_compose_envs() -> [(&'static str, &'static str); 2] {
    [
        ("CN_POSTGRES_PASSWORD", "cn_password"),
        ("COMMUNITY_NODE_JWT_SECRET", "xtask-test-secret"),
    ]
}

fn cn_test_envs() -> [(&'static str, &'static str); 2] {
    [
        ("KUKURI_CN_RUN_INTEGRATION_TESTS", "1"),
        (
            "COMMUNITY_NODE_DATABASE_URL",
            "postgres://cn:cn_password@127.0.0.1:55432/cn",
        ),
    ]
}

fn with_cn_postgres<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let root = root_dir();
    run_with_env(
        "docker",
        [
            "compose",
            "-f",
            "docker-compose.community-node.yml",
            "up",
            "-d",
            "--wait",
            "cn-postgres",
        ],
        &root,
        &cn_compose_envs(),
    )?;
    let operation_result = operation();
    let shutdown_result = run_with_env(
        "docker",
        [
            "compose",
            "-f",
            "docker-compose.community-node.yml",
            "down",
            "-v",
            "--remove-orphans",
        ],
        &root,
        &cn_compose_envs(),
    );
    match (operation_result, shutdown_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(operation_error), Err(shutdown_error)) => Err(operation_error.context(format!(
            "failed to tear down cn-postgres after error: {shutdown_error:#}"
        ))),
    }
}

fn scenario_requires_cn_postgres(name: &str) -> bool {
    matches!(
        name,
        "community_node_public_connectivity" | "community_node_multi_device_connectivity"
    )
}
