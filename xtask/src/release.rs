use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

#[allow(unused_imports)]
use crate::*;

pub(crate) fn release_check(tag: Option<&str>) -> Result<()> {
    let root = root_dir();
    asset_check()?;
    let workspace_version = read_workspace_version(&root.join("Cargo.toml"))?;
    let tauri_version = read_package_version(&desktop_dir().join("src-tauri").join("Cargo.toml"))?;
    let desktop_package_version = read_json_version(&desktop_dir().join("package.json"))
        .context("desktop package version")?;
    let tauri_config_version =
        read_json_version(&desktop_dir().join("src-tauri").join("tauri.conf.json"))
            .context("tauri config version")?;

    for (label, version) in [
        ("apps/desktop/src-tauri/Cargo.toml", tauri_version.as_str()),
        (
            "apps/desktop/package.json",
            desktop_package_version.as_str(),
        ),
        (
            "apps/desktop/src-tauri/tauri.conf.json",
            tauri_config_version.as_str(),
        ),
    ] {
        if version != workspace_version {
            bail!(
                "release version mismatch: workspace version is {workspace_version}, {label} has {version}"
            );
        }
    }

    if let Some(tag) = tag {
        validate_release_tag(&workspace_version, tag)?;
    }

    println!(
        "[xtask] release version ok: workspace={workspace_version} channel=preview tag={}",
        tag.unwrap_or("<not checked>")
    );
    Ok(())
}

pub(crate) fn validate_release_tag(workspace_version: &str, tag: &str) -> Result<()> {
    let expected = format!("v{workspace_version}-preview.");
    if !tag.starts_with(&expected) {
        bail!("release tag must start with {expected} and include a preview number, got {tag}");
    }
    let suffix = &tag[expected.len()..];
    if suffix.is_empty() || !suffix.chars().all(|value| value.is_ascii_digit()) {
        bail!("release tag preview suffix must be numeric, got {tag}");
    }
    Ok(())
}

pub(crate) fn read_package_version(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]" || trimmed == "[workspace.package]";
            continue;
        }
        if in_package && trimmed.starts_with("version") {
            return parse_toml_string_value(trimmed)
                .with_context(|| format!("failed to parse version in {}", path.display()));
        }
    }
    bail!("version was not found in {}", path.display())
}

pub(crate) fn read_workspace_version(path: &Path) -> Result<String> {
    read_package_version(path)
}

pub(crate) fn parse_toml_string_value(line: &str) -> Result<String> {
    let (_, value) = line
        .split_once('=')
        .context("expected key = \"value\" TOML line")?;
    let value = value.trim();
    if !(value.starts_with('"') && value.ends_with('"') && value.len() >= 2) {
        bail!("expected quoted TOML string value");
    }
    Ok(value[1..value.len() - 1].to_string())
}

pub(crate) fn read_json_version(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    value
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("version was not found in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tag_accepts_preview_tag_matching_workspace_version() {
        assert!(validate_release_tag("0.1.2", "v0.1.2-preview.1").is_ok());
        assert!(validate_release_tag("0.1.2", "v0.1.2-preview.12").is_ok());
    }

    #[test]
    fn release_tag_rejects_version_mismatch_and_bad_suffixes() {
        assert!(validate_release_tag("0.1.2", "v0.1.3-preview.1").is_err());
        assert!(validate_release_tag("0.1.2", "v0.1.2-preview.").is_err());
        assert!(validate_release_tag("0.1.2", "v0.1.2-preview.1a").is_err());
        assert!(validate_release_tag("0.1.2", "v0.1.2").is_err());
    }

    #[test]
    fn parse_toml_string_value_extracts_quoted_values_only() {
        assert_eq!(
            parse_toml_string_value("version = \"0.1.2\"").unwrap(),
            "0.1.2"
        );
        assert!(parse_toml_string_value("version = 3").is_err());
        assert!(parse_toml_string_value("no equals sign").is_err());
    }
}
