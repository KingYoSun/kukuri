use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::root_dir;

const OPERATOR_SPECIFIC_VALUES: [&str; 3] =
    ["ops@kukuri.app", "api.kukuri.app", "iroh-relay.kukuri.app"];

const SCAN_ROOTS: [&str; 8] = [
    "infra/terraform",
    "docker-compose.community-node.yml",
    "scripts/vps",
    "crates/desktop-runtime/src",
    "apps/desktop/src",
    "apps/desktop/src-tauri",
    "docs/runbooks/community-node-self-host-vps.md",
    "docs/runbooks/community-node-production-rollout.md",
];

pub(crate) fn operator_neutrality_check() -> Result<()> {
    let repository = root_dir();
    let mut violations = Vec::new();
    for relative_root in SCAN_ROOTS {
        collect_violations(
            &repository,
            &repository.join(relative_root),
            &mut violations,
        )?;
    }
    validate_https_community_node_csp(&repository)?;
    if violations.is_empty() {
        return Ok(());
    }

    bail!(
        "operator-specific values must live only in distribution config, tests/fixtures, or the default-node runbook:\n{}",
        violations.join("\n")
    )
}

fn validate_https_community_node_csp(repository: &Path) -> Result<()> {
    let path = repository.join("apps/desktop/src-tauri/tauri.conf.json");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let connect_src = config
        .pointer("/app/security/csp/connect-src")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !connect_src
        .split_ascii_whitespace()
        .any(|source| source == "https:")
    {
        bail!(
            "apps/desktop/src-tauri/tauri.conf.json connect-src must allow operator-neutral HTTPS Community Nodes"
        );
    }
    Ok(())
}

fn collect_violations(repository: &Path, path: &Path, violations: &mut Vec<String>) -> Result<()> {
    let relative = path.strip_prefix(repository).unwrap_or(path);
    if is_allowlisted(relative) {
        return Ok(());
    }
    if path.is_dir() {
        for entry in
            fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?
        {
            collect_violations(repository, &entry?.path(), violations)?;
        }
        return Ok(());
    }
    if !path.is_file() {
        return Ok(());
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    for (line_index, line) in content.lines().enumerate() {
        for value in violations_in_text(line) {
            violations.push(format!(
                "{}:{} contains `{value}`",
                relative.display(),
                line_index + 1
            ));
        }
    }
    Ok(())
}

fn violations_in_text(text: &str) -> impl Iterator<Item = &'static str> + '_ {
    OPERATOR_SPECIFIC_VALUES
        .into_iter()
        .filter(|value| text.contains(value))
}

fn is_allowlisted(relative: &Path) -> bool {
    let normalized = relative.to_string_lossy().replace('\\', "/");
    normalized == "apps/desktop/src-tauri/distribution/community-nodes.json"
        || normalized == "apps/desktop/src-tauri/distribution/legal.json"
        || normalized == "docs/runbooks/community-node-production-rollout.md"
        || normalized.ends_with("/.terraform")
        || normalized.contains("/.terraform/")
        || normalized.ends_with("/target")
        || normalized.contains("/target/")
        || normalized.ends_with("/terraform.tfvars")
        || normalized.ends_with(".bak")
        || normalized.ends_with(".tfplan")
        || normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.contains("/fixtures/")
        || normalized.contains("/mocks/")
        || normalized.contains("/stories/")
        || normalized.ends_with(".test.ts")
        || normalized.ends_with(".test.tsx")
        || normalized.ends_with("/fixtures.ts")
        || normalized.ends_with(".stories.ts")
        || normalized.ends_with(".stories.tsx")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_each_operator_specific_value() {
        let text = "ops@kukuri.app https://api.kukuri.app https://iroh-relay.kukuri.app";
        assert_eq!(
            violations_in_text(text).collect::<Vec<_>>(),
            OPERATOR_SPECIFIC_VALUES
        );
    }

    #[test]
    fn allowlist_is_limited_to_distribution_and_non_product_evidence() {
        assert!(is_allowlisted(Path::new(
            "apps/desktop/src-tauri/distribution/community-nodes.json"
        )));
        assert!(is_allowlisted(Path::new(
            "apps/desktop/src-tauri/distribution/legal.json"
        )));
        assert!(is_allowlisted(Path::new(
            "docs/runbooks/community-node-production-rollout.md"
        )));
        assert!(is_allowlisted(Path::new(
            "apps/desktop/src/components/example.test.tsx"
        )));
        assert!(is_allowlisted(Path::new(
            "crates/desktop-runtime/src/tests/community_node/config.rs"
        )));
        assert!(!is_allowlisted(Path::new(
            "crates/desktop-runtime/src/community_node/config_support.rs"
        )));
        assert!(!is_allowlisted(Path::new(
            "infra/terraform/envs/low-cost/variables.tf"
        )));
    }

    #[test]
    fn generic_example_values_are_neutral() {
        assert!(
            violations_in_text("admin@example.com api.example.com")
                .next()
                .is_none()
        );
    }
}
