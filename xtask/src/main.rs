use anyhow::{Context, Result, bail};

mod cn;
mod desktop;
mod exec;
mod ipc;
mod oversized;
mod release;
mod rust;
mod scenario;

pub(crate) use cn::*;
pub(crate) use desktop::*;
pub(crate) use exec::*;
pub(crate) use ipc::*;
pub(crate) use oversized::*;
pub(crate) use release::*;
pub(crate) use rust::*;
pub(crate) use scenario::*;

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
        "app-api-slow-test" => app_api_slow_test(),
        "tauri-check" => tauri_check(),
        "desktop-lint" => desktop_lint(),
        "desktop-test" => desktop_test(),
        "desktop-storybook" => desktop_storybook(),
        "desktop-browser-test" => desktop_browser_test(),
        "desktop-visual-test" => desktop_visual_test(),
        "desktop-ui-check" => desktop_ui_check(),
        "cn-check" => cn_check(),
        "cn-test" => cn_test(),
        "desktop-package" => desktop_package(),
        "release-check" => {
            let tag = args.next();
            release_check(tag.as_deref())
        }
        "oversized-files" => {
            let update_baseline = match args.next().as_deref() {
                None => false,
                Some("--update-baseline") => true,
                Some(flag) => {
                    print_usage();
                    bail!("unsupported oversized-files flag: {flag}");
                }
            };
            oversized_files(update_baseline)
        }
        "ipc-types" => {
            let check = match args.next().as_deref() {
                None => false,
                Some("--check") => true,
                Some(flag) => {
                    print_usage();
                    bail!("unsupported ipc-types flag: {flag}");
                }
            };
            ipc_types(check)
        }
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

fn print_usage() {
    eprintln!(
        "usage: cargo xtask <doctor|check|test|rust-check|rust-test|app-api-slow-test|tauri-check|desktop-lint|desktop-test|desktop-storybook|desktop-browser-test|desktop-visual-test|desktop-ui-check|cn-check|cn-test|desktop-package|release-check [tag]|oversized-files [--update-baseline]|ipc-types [--check]|e2e-smoke|scenario <name>>"
    );
}
