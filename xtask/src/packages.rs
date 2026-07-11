use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

const CN_LANE_METADATA_PATH: [&str; 2] = ["kukuri", "cn-lane"];

pub(crate) fn cn_packages() -> Result<Vec<String>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(crate::root_dir())
        .output()
        .context("failed to execute cargo metadata for CN lane discovery")?;
    if !output.status.success() {
        bail!(
            "cargo metadata for CN lane discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .context("cargo metadata returned invalid JSON for CN lane discovery")?;
    cn_packages_from_metadata(&metadata)
}

fn cn_packages_from_metadata(metadata: &Value) -> Result<Vec<String>> {
    let packages = metadata["packages"]
        .as_array()
        .context("cargo metadata is missing the packages array")?;
    let mut selected = Vec::new();
    for package in packages {
        let name = package["name"]
            .as_str()
            .context("cargo metadata package is missing a string name")?;
        let marker = &package["metadata"][CN_LANE_METADATA_PATH[0]][CN_LANE_METADATA_PATH[1]];
        if marker.is_null() {
            continue;
        }
        let enabled = marker.as_bool().with_context(|| {
            format!(
                "package `{name}` metadata.{}.{} must be boolean",
                CN_LANE_METADATA_PATH[0], CN_LANE_METADATA_PATH[1]
            )
        })?;
        if enabled {
            selected.push(name.to_string());
        }
    }
    selected.sort();
    if selected.is_empty() {
        bail!("cargo metadata selected no CN lane packages");
    }
    if selected.windows(2).any(|pair| pair[0] == pair[1]) {
        bail!("cargo metadata selected duplicate CN lane package names");
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selects_only_explicit_cn_lane_packages_in_name_order() {
        let metadata = json!({
            "packages": [
                {"name": "kukuri-cn-z", "metadata": {"kukuri": {"cn-lane": true}}},
                {"name": "kukuri-cn-protocol", "metadata": {}},
                {"name": "kukuri-cn-a", "metadata": {"kukuri": {"cn-lane": true}}},
                {"name": "kukuri-cn-arachnid", "metadata": {"kukuri": {"cn-lane": false}}}
            ]
        });
        assert_eq!(
            cn_packages_from_metadata(&metadata).unwrap(),
            vec!["kukuri-cn-a".to_string(), "kukuri-cn-z".to_string()]
        );
    }

    #[test]
    fn rejects_non_boolean_marker() {
        let metadata = json!({
            "packages": [{"name": "bad", "metadata": {"kukuri": {"cn-lane": "yes"}}}]
        });
        assert!(
            cn_packages_from_metadata(&metadata)
                .unwrap_err()
                .to_string()
                .contains("must be boolean")
        );
    }

    #[test]
    fn rejects_empty_selection() {
        let metadata = json!({"packages": [{"name": "regular", "metadata": {}}]});
        assert!(
            cn_packages_from_metadata(&metadata)
                .unwrap_err()
                .to_string()
                .contains("selected no CN lane packages")
        );
    }
}
