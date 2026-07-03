use anyhow::{Result, bail};

#[allow(unused_imports)]
use crate::*;

pub(crate) const TAURI_CHECK_TARGET_DIR: &str = "target/desktop-tauri-check";

pub(crate) fn tauri_check() -> Result<()> {
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

pub(crate) fn desktop_lint() -> Result<()> {
    run_pnpm(["lint"], &desktop_dir())?;
    run_pnpm(["typecheck"], &desktop_dir())
}

pub(crate) fn desktop_test() -> Result<()> {
    run_pnpm(["test"], &desktop_dir())
}

pub(crate) fn desktop_storybook() -> Result<()> {
    run_pnpm(["storybook:build"], &desktop_dir())
}

pub(crate) fn desktop_browser_test() -> Result<()> {
    run_pnpm(["test:e2e:browser"], &desktop_dir())
}

pub(crate) fn desktop_ui_check() -> Result<()> {
    desktop_lint()?;
    desktop_test()?;
    desktop_storybook()?;
    desktop_browser_test()
}

pub(crate) fn desktop_package() -> Result<()> {
    if !cfg!(target_os = "windows") {
        bail!("desktop-package is only supported on Windows hosts");
    }

    let mut args = vec![
        "tauri".to_string(),
        "build".to_string(),
        "--target".to_string(),
        "x86_64-pc-windows-msvc".to_string(),
    ];
    if std::env::var_os("TAURI_SIGNING_PRIVATE_KEY").is_none() {
        println!(
            "[xtask] TAURI_SIGNING_PRIVATE_KEY is not set; building installer without updater artifacts"
        );
        args.extend([
            "--config".to_string(),
            r#"{"bundle":{"createUpdaterArtifacts":false}}"#.to_string(),
        ]);
    }

    run_pnpm(args, &desktop_dir())
}
