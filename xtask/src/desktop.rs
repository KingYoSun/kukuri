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

pub(crate) fn desktop_visual_test() -> Result<()> {
    run_pnpm(["test:e2e:visual"], &desktop_dir())
}

pub(crate) fn desktop_ui_check() -> Result<()> {
    desktop_lint()?;
    desktop_test()?;
    desktop_storybook()?;
    desktop_browser_test()?;
    desktop_visual_test()
}

pub(crate) fn desktop_package() -> Result<()> {
    let signed = std::env::var_os("TAURI_SIGNING_PRIVATE_KEY")
        .is_some_and(|key| !cfg!(target_os = "linux") || !key.is_empty());
    if cfg!(target_os = "linux") && std::env::consts::ARCH != "x86_64" {
        bail!("Linux desktop-package requires an x86_64 host");
    }
    let args = desktop_package_args(std::env::consts::OS, signed)?;
    if !signed {
        println!(
            "[xtask] TAURI_SIGNING_PRIVATE_KEY is not set; building installer without updater artifacts"
        );
    }
    run_pnpm(args, &desktop_dir())?;
    if cfg!(target_os = "linux") {
        crate::appimage::verify_package()?;
    }
    Ok(())
}

fn desktop_package_args(os: &str, signed: bool) -> Result<Vec<String>> {
    let target = match os {
        "windows" => "x86_64-pc-windows-msvc",
        "linux" if signed => "x86_64-unknown-linux-gnu",
        "linux" => bail!("Linux desktop-package requires TAURI_SIGNING_PRIVATE_KEY"),
        _ => bail!("desktop-package is only supported on Windows and Linux hosts"),
    };

    let mut args = vec![
        "tauri".to_string(),
        "build".to_string(),
        "--target".to_string(),
        target.to_string(),
    ];
    if !signed {
        args.extend([
            "--config".to_string(),
            r#"{"bundle":{"createUpdaterArtifacts":false}}"#.to_string(),
        ]);
    }

    Ok(args)
}

#[cfg(test)]
mod package_tests {
    use super::*;

    #[test]
    fn linux_package_uses_appimage_target_and_keeps_updater_signing() {
        let args = desktop_package_args("linux", true).expect("Linux package is supported");
        assert!(args.iter().any(|value| value == "x86_64-unknown-linux-gnu"));
        assert!(!args.iter().any(|value| value.contains("false")));
    }

    #[test]
    fn linux_package_rejects_missing_signing_key() {
        assert!(desktop_package_args("linux", false).is_err());
    }

    #[test]
    fn windows_package_preserves_signed_and_local_unsigned_builds() {
        for signed in [true, false] {
            let args = desktop_package_args("windows", signed).unwrap();
            assert!(args.iter().any(|value| value == "x86_64-pc-windows-msvc"));
            assert_eq!(args.iter().any(|value| value.contains("false")), !signed);
        }
    }

    #[test]
    fn unsupported_package_host_is_rejected() {
        assert!(desktop_package_args("macos", true).is_err());
    }

    #[test]
    fn linux_bundle_config_enables_only_signed_appimage() {
        let config: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root_dir().join("apps/desktop/src-tauri/tauri.linux.conf.json"))
                .expect("Linux bundle config exists"),
        )
        .unwrap();
        assert_eq!(config["bundle"]["active"], true);
        assert_eq!(config["bundle"]["targets"], "appimage");
        assert_eq!(config["bundle"]["createUpdaterArtifacts"], true);
        assert!(
            config["bundle"]["icon"]
                .as_array()
                .is_some_and(|icons| !icons.is_empty())
        );
        assert_eq!(config["bundle"]["category"], "SocialNetworking");
    }
}
