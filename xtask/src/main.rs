use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

const CN_PACKAGES: [&str; 4] = [
    "kukuri-cn-core",
    "kukuri-cn-user-api",
    "kukuri-cn-iroh-relay",
    "kukuri-cn-cli",
];
const SERIAL_RUST_PACKAGE: &str = "kukuri-harness";
const TAURI_CHECK_TARGET_DIR: &str = "target/desktop-tauri-check";
const PNPM_VERSION: &str = "10.16.1";
const OVERSIZED_FILE_LINE_LIMIT: usize = 1000;
const OVERSIZED_FILE_EXTENSIONS: &[&str] = &[
    "css", "html", "js", "json", "jsx", "md", "ps1", "rs", "scss", "sh", "sql", "toml", "ts",
    "tsx", "txt", "yaml", "yml",
];
const OVERSIZED_FILE_EXCLUDED_PATH_PREFIXES: &[&str] = &[
    "apps/desktop/src-tauri/gen/",
    "apps/desktop/src-tauri/icons/",
];
const OVERSIZED_FILE_EXCLUDED_EXACT_PATHS: &[&str] = &[
    "Cargo.lock",
    "apps/desktop/pnpm-lock.yaml",
    "apps/desktop/src-tauri/Cargo.lock",
];

static NEXTEST_AVAILABLE: OnceLock<bool> = OnceLock::new();
static PNPM_AVAILABLE: OnceLock<bool> = OnceLock::new();

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
        "rust-check" => rust_check(),
        "rust-test" => rust_test(),
        "tauri-check" => tauri_check(),
        "desktop-lint" => desktop_lint(),
        "desktop-test" => desktop_test(),
        "desktop-storybook" => desktop_storybook(),
        "desktop-browser-test" => desktop_browser_test(),
        "desktop-ui-check" => desktop_ui_check(),
        "cn-check" => cn_check(),
        "cn-test" => cn_test(),
        "desktop-package" => desktop_package(),
        "oversized-files" => oversized_files(),
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
    rust_check()?;
    tauri_check()?;
    desktop_lint()
}

fn test() -> Result<()> {
    rust_test()?;
    desktop_test()
}

fn rust_check() -> Result<()> {
    run("cargo", ["fmt", "--check"], &root_dir())?;

    let mut clippy_args = vec![
        "clippy".to_string(),
        "--workspace".to_string(),
        "--all-targets".to_string(),
    ];
    clippy_args.extend(cargo_exclude_args(&CN_PACKAGES));
    clippy_args.extend(["--".to_string(), "-D".to_string(), "warnings".to_string()]);
    run("cargo", clippy_args, &root_dir())
}

fn rust_test() -> Result<()> {
    if nextest_available() {
        rust_test_with_nextest()
    } else {
        if is_ci() {
            bail!("cargo-nextest is required in CI");
        }
        eprintln!(
            "[xtask] warning: cargo-nextest was not found; falling back to cargo test for rust-test"
        );
        rust_test_with_cargo_test()
    }
}

fn rust_test_with_nextest() -> Result<()> {
    let mut nextest_args = vec![
        "nextest".to_string(),
        "run".to_string(),
        "--workspace".to_string(),
    ];
    nextest_args.extend(cargo_exclude_args(
        &[&CN_PACKAGES[..], &[SERIAL_RUST_PACKAGE]].concat(),
    ));
    run("cargo", nextest_args, &root_dir())?;

    run(
        "cargo",
        ["nextest", "run", "-p", SERIAL_RUST_PACKAGE, "-j", "1"],
        &root_dir(),
    )?;

    let mut doc_args = vec![
        "test".to_string(),
        "--workspace".to_string(),
        "--doc".to_string(),
    ];
    doc_args.extend(cargo_exclude_args(&CN_PACKAGES));
    run("cargo", doc_args, &root_dir())
}

fn rust_test_with_cargo_test() -> Result<()> {
    let mut regular_test_args = vec![
        "test".to_string(),
        "--workspace".to_string(),
        "--lib".to_string(),
        "--bins".to_string(),
        "--tests".to_string(),
    ];
    regular_test_args.extend(cargo_exclude_args(
        &[&CN_PACKAGES[..], &[SERIAL_RUST_PACKAGE]].concat(),
    ));
    run("cargo", regular_test_args, &root_dir())?;

    run_with_env(
        "cargo",
        [
            "test",
            "-p",
            SERIAL_RUST_PACKAGE,
            "--lib",
            "--bins",
            "--tests",
        ],
        &root_dir(),
        &[("RUST_TEST_THREADS", "1")],
    )?;

    let mut doc_args = vec![
        "test".to_string(),
        "--workspace".to_string(),
        "--doc".to_string(),
    ];
    doc_args.extend(cargo_exclude_args(&CN_PACKAGES));
    run("cargo", doc_args, &root_dir())
}

fn tauri_check() -> Result<()> {
    let target_dir = root_dir().join(TAURI_CHECK_TARGET_DIR);
    let target_dir_value = target_dir.to_string_lossy().into_owned();
    run_with_env(
        "cargo",
        [
            "check",
            "--manifest-path",
            "apps/desktop/src-tauri/Cargo.toml",
        ],
        &root_dir(),
        &[("CARGO_TARGET_DIR", target_dir_value.as_str())],
    )
}

fn desktop_lint() -> Result<()> {
    run_pnpm(["lint"], &desktop_dir())?;
    run_pnpm(["typecheck"], &desktop_dir())
}

fn desktop_test() -> Result<()> {
    run_pnpm(["test"], &desktop_dir())
}

fn desktop_storybook() -> Result<()> {
    run_pnpm(["storybook:build"], &desktop_dir())
}

fn desktop_browser_test() -> Result<()> {
    run_pnpm(["test:e2e:browser"], &desktop_dir())
}

fn desktop_ui_check() -> Result<()> {
    desktop_lint()?;
    desktop_test()?;
    desktop_storybook()?;
    desktop_browser_test()
}

fn cn_check() -> Result<()> {
    run(
        "cargo",
        cargo_package_args("check", &CN_PACKAGES),
        &root_dir(),
    )
}

fn cn_test() -> Result<()> {
    with_cn_postgres(|| {
        run_with_env(
            "cargo",
            cargo_package_args("test", &CN_PACKAGES),
            &root_dir(),
            &cn_test_envs(),
        )
    })
}

fn e2e_smoke(name: &str) -> Result<()> {
    run_timed_step(format!("scenario {name}"), || {
        let root = root_dir();
        let artifacts = artifacts_dir(name);
        let name = name.to_string();
        let handle = std::thread::Builder::new()
            .name(format!("scenario-{name}"))
            .stack_size(64 * 1024 * 1024)
            .spawn(move || -> Result<_> {
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to build tokio runtime")?;
                runtime.block_on(kukuri_harness::run_named_scenario(
                    &root,
                    name.as_str(),
                    &artifacts,
                ))
            })
            .context("failed to spawn scenario runner thread")?;
        let result = handle
            .join()
            .map_err(|_| anyhow::anyhow!("scenario runner thread panicked"))??;
        let metrics = kukuri_harness::summarize_metrics(&result);
        for (key, value) in metrics {
            println!("{key}={value}");
        }
        Ok(())
    })
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct OversizedFileReport {
    path: String,
    line_count: usize,
    category: &'static str,
}

fn oversized_files() -> Result<()> {
    let reports = collect_oversized_files(&root_dir())?;
    if reports.is_empty() {
        println!(
            "[xtask] no oversized tracked hand-written files found (threshold={} lines)",
            OVERSIZED_FILE_LINE_LIMIT
        );
        return Ok(());
    }

    eprintln!(
        "[xtask] warning: found {} oversized tracked hand-written files (threshold={} lines)",
        reports.len(),
        OVERSIZED_FILE_LINE_LIMIT
    );
    for report in reports {
        println!(
            "[xtask] warning oversized-file category={} lines={} path={}",
            report.category, report.line_count, report.path
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostPlatform {
    Unix,
    Windows,
}

#[derive(Debug, Eq, PartialEq)]
struct CommandSpec {
    program: String,
    args: Vec<String>,
}

impl CommandSpec {
    fn direct(binary: &str, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            program: binary.to_string(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

fn host_platform() -> HostPlatform {
    if cfg!(windows) {
        HostPlatform::Windows
    } else {
        HostPlatform::Unix
    }
}

fn node_command_spec(
    platform: HostPlatform,
    binary: &str,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> CommandSpec {
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    match platform {
        HostPlatform::Unix => CommandSpec::direct(binary, args),
        HostPlatform::Windows => {
            let mut shell_args = vec!["/C".to_string(), binary.to_string()];
            shell_args.extend(args);
            CommandSpec::direct("cmd", shell_args)
        }
    }
}

fn pnpm_command_spec(
    platform: HostPlatform,
    pnpm_available: bool,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> CommandSpec {
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    if pnpm_available {
        node_command_spec(platform, "pnpm", args)
    } else {
        let mut fallback = match platform {
            HostPlatform::Unix => vec![format!("pnpm@{PNPM_VERSION}")],
            HostPlatform::Windows => vec!["--yes".to_string(), format!("pnpm@{PNPM_VERSION}")],
        };
        fallback.extend(args);
        node_command_spec(platform, "npx", fallback)
    }
}

fn run(binary: &str, args: impl IntoIterator<Item = impl Into<String>>, cwd: &Path) -> Result<()> {
    run_with_env(binary, args, cwd, &[])
}

fn run_with_env(
    binary: &str,
    args: impl IntoIterator<Item = impl Into<String>>,
    cwd: &Path,
    envs: &[(&str, &str)],
) -> Result<()> {
    run_spec_with_env(&CommandSpec::direct(binary, args), cwd, envs)
}

fn run_spec_with_env(spec: &CommandSpec, cwd: &Path, envs: &[(&str, &str)]) -> Result<()> {
    let label = format_command(spec);
    run_timed_step(label, || {
        let status = Command::new(spec.program.as_str())
            .args(spec.args.iter())
            .current_dir(cwd)
            .envs(envs.iter().copied())
            .status()
            .with_context(|| format!("failed to execute {}", spec.program))?;
        if !status.success() {
            bail!("{} exited with status {status}", spec.program);
        }
        Ok(())
    })
}

fn run_capture(
    binary: &str,
    args: impl IntoIterator<Item = impl Into<String>>,
    cwd: &Path,
) -> Result<()> {
    run_capture_spec(&CommandSpec::direct(binary, args), cwd)
}

fn run_capture_spec(spec: &CommandSpec, cwd: &Path) -> Result<()> {
    let label = format_command(spec);
    run_timed_step(label, || {
        let output = Command::new(spec.program.as_str())
            .args(spec.args.iter())
            .current_dir(cwd)
            .output()
            .with_context(|| format!("failed to execute {}", spec.program))?;
        if !output.status.success() {
            bail!("{} exited with status {}", spec.program, output.status);
        }
        Ok(())
    })
}

fn run_pnpm(args: impl IntoIterator<Item = impl Into<String>>, cwd: &Path) -> Result<()> {
    let platform = host_platform();
    let available = *PNPM_AVAILABLE.get_or_init(|| {
        run_capture_spec(&node_command_spec(platform, "pnpm", ["--version"]), cwd).is_ok()
    });
    run_spec_with_env(&pnpm_command_spec(platform, available, args), cwd, &[])
}

fn run_timed_step<T>(label: impl Into<String>, operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let label = label.into();
    println!("[xtask] start {label}");
    let started = Instant::now();
    match operation() {
        Ok(value) => {
            println!(
                "[xtask] done  {label} ({})",
                format_duration(started.elapsed())
            );
            Ok(value)
        }
        Err(error) => {
            eprintln!(
                "[xtask] fail  {label} ({})",
                format_duration(started.elapsed())
            );
            Err(error)
        }
    }
}

fn print_usage() {
    eprintln!(
        "usage: cargo xtask <doctor|check|test|rust-check|rust-test|tauri-check|desktop-lint|desktop-test|desktop-storybook|desktop-browser-test|desktop-ui-check|cn-check|cn-test|desktop-package|oversized-files|e2e-smoke|scenario <name>>"
    );
}

fn cargo_exclude_args(packages: &[&str]) -> Vec<String> {
    let mut args = Vec::with_capacity(packages.len() * 2);
    for package in packages {
        args.push("--exclude".to_string());
        args.push((*package).to_string());
    }
    args
}

fn cargo_package_args(command: &str, packages: &[&str]) -> Vec<String> {
    let mut args = Vec::with_capacity(1 + packages.len() * 2);
    args.push(command.to_string());
    for package in packages {
        args.push("-p".to_string());
        args.push((*package).to_string());
    }
    args
}

fn format_command(spec: &CommandSpec) -> String {
    if spec.args.is_empty() {
        spec.program.clone()
    } else {
        format!("{} {}", spec.program, spec.args.join(" "))
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() >= 60 {
        let total_seconds = duration.as_secs();
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}m{seconds:02}s")
    } else if duration.as_secs_f64() >= 1.0 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

fn nextest_available() -> bool {
    *NEXTEST_AVAILABLE
        .get_or_init(|| run_capture("cargo", ["nextest", "--version"], &root_dir()).is_ok())
}

fn is_ci() -> bool {
    std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some()
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
    if std::env::var_os("KUKURI_HARNESS_COMMUNITY_NODE_BASE_URL").is_some() {
        return false;
    }
    matches!(
        name,
        "community_node_public_connectivity" | "community_node_multi_device_connectivity"
    )
}

fn collect_oversized_files(root: &Path) -> Result<Vec<OversizedFileReport>> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .current_dir(root)
        .output()
        .context("failed to execute git ls-files for oversized file report")?;
    if !output.status.success() {
        bail!("git ls-files exited with status {}", output.status);
    }

    let mut reports = Vec::new();
    for entry in output.stdout.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let relative_path =
            std::str::from_utf8(entry).context("git ls-files returned a non-utf8 path")?;
        if !should_scan_oversized_file(relative_path) {
            continue;
        }

        let file_path = root.join(relative_path);
        let contents = match std::fs::read_to_string(&file_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to read hand-written file for oversized report: {}",
                        file_path.display()
                    )
                });
            }
        };
        let line_count = contents.lines().count();
        if line_count >= OVERSIZED_FILE_LINE_LIMIT {
            reports.push(OversizedFileReport {
                path: relative_path.replace('\\', "/"),
                line_count,
                category: oversized_file_category(relative_path),
            });
        }
    }

    reports.sort_by(|left, right| {
        right
            .line_count
            .cmp(&left.line_count)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(reports)
}

fn should_scan_oversized_file(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    if OVERSIZED_FILE_EXCLUDED_EXACT_PATHS.contains(&normalized.as_str()) {
        return false;
    }
    if OVERSIZED_FILE_EXCLUDED_PATH_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return false;
    }
    let extension = Path::new(normalized.as_str())
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    let Some(extension) = extension else {
        return false;
    };
    OVERSIZED_FILE_EXTENSIONS.contains(&extension.as_str())
}

fn oversized_file_category(path: &str) -> &'static str {
    let normalized = path.replace('\\', "/");
    if normalized.contains("/tests/")
        || normalized.ends_with(".test.ts")
        || normalized.ends_with(".test.tsx")
        || normalized.ends_with(".test.rs")
    {
        "test"
    } else if normalized.contains("/mocks/")
        || normalized.contains("/styles/")
        || normalized.contains("/scenarios/")
        || normalized.ends_with("/waiters.rs")
    {
        "support"
    } else {
        "production"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CN_PACKAGES, CommandSpec, HostPlatform, OVERSIZED_FILE_EXCLUDED_EXACT_PATHS,
        OVERSIZED_FILE_EXCLUDED_PATH_PREFIXES, PNPM_VERSION, cargo_exclude_args, node_command_spec,
        oversized_file_category, pnpm_command_spec, should_scan_oversized_file,
    };

    #[test]
    fn node_command_uses_direct_exec_on_unix() {
        let spec = node_command_spec(HostPlatform::Unix, "pnpm", ["test"]);
        assert_eq!(
            spec,
            CommandSpec {
                program: "pnpm".to_string(),
                args: vec!["test".to_string()],
            }
        );
    }

    #[test]
    fn node_command_uses_cmd_shell_on_windows() {
        let spec = node_command_spec(HostPlatform::Windows, "npx", ["--yes", "pnpm@10.16.1"]);
        assert_eq!(
            spec,
            CommandSpec {
                program: "cmd".to_string(),
                args: vec![
                    "/C".to_string(),
                    "npx".to_string(),
                    "--yes".to_string(),
                    "pnpm@10.16.1".to_string(),
                ],
            }
        );
    }

    #[test]
    fn pnpm_fallback_uses_npx_wrapper_when_pnpm_is_unavailable() {
        let spec = pnpm_command_spec(HostPlatform::Windows, false, ["lint"]);
        assert_eq!(
            spec,
            CommandSpec {
                program: "cmd".to_string(),
                args: vec![
                    "/C".to_string(),
                    "npx".to_string(),
                    "--yes".to_string(),
                    format!("pnpm@{PNPM_VERSION}"),
                    "lint".to_string(),
                ],
            }
        );
    }

    #[test]
    fn cargo_exclude_args_expands_each_package() {
        assert_eq!(
            cargo_exclude_args(&CN_PACKAGES[..2]),
            vec![
                "--exclude".to_string(),
                CN_PACKAGES[0].to_string(),
                "--exclude".to_string(),
                CN_PACKAGES[1].to_string(),
            ]
        );
    }

    #[test]
    fn oversized_file_report_skips_known_generated_and_lock_files() {
        for path in OVERSIZED_FILE_EXCLUDED_EXACT_PATHS {
            assert!(!should_scan_oversized_file(path));
        }
        for prefix in OVERSIZED_FILE_EXCLUDED_PATH_PREFIXES {
            assert!(!should_scan_oversized_file(&format!("{prefix}example.rs")));
        }
    }

    #[test]
    fn oversized_file_report_scans_hand_written_source_files() {
        assert!(should_scan_oversized_file("crates/app-api/src/service.rs"));
        assert!(should_scan_oversized_file(
            "apps/desktop/src/shell/DesktopShellPage.tsx"
        ));
        assert!(!should_scan_oversized_file("apps/desktop/app-icon.png"));
    }

    #[test]
    fn oversized_file_report_categories_match_expected_paths() {
        assert_eq!(
            oversized_file_category("crates/app-api/src/service.rs"),
            "production"
        );
        assert_eq!(
            oversized_file_category("apps/desktop/src/shell/DesktopShellPage.test.tsx"),
            "test"
        );
        assert_eq!(
            oversized_file_category("apps/desktop/src/styles/shell-phase1.css"),
            "support"
        );
    }
}
