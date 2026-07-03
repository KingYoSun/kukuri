use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

#[allow(unused_imports)]
use crate::*;

pub(crate) const OVERSIZED_FILE_LINE_LIMIT: usize = 1000;
pub(crate) const OVERSIZED_FILE_EXTENSIONS: &[&str] = &[
    "css", "html", "js", "json", "jsx", "md", "ps1", "rs", "scss", "sh", "sql", "toml", "ts",
    "tsx", "txt", "yaml", "yml",
];
pub(crate) const OVERSIZED_FILE_EXCLUDED_PATH_PREFIXES: &[&str] = &[
    "apps/desktop/src-tauri/gen/",
    "apps/desktop/src-tauri/icons/",
];
pub(crate) const OVERSIZED_FILE_EXCLUDED_EXACT_PATHS: &[&str] = &[
    "Cargo.lock",
    "apps/desktop/pnpm-lock.yaml",
    "apps/desktop/src-tauri/Cargo.lock",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OversizedFileReport {
    path: String,
    line_count: usize,
    category: &'static str,
}

pub(crate) fn oversized_files() -> Result<()> {
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

pub(crate) fn collect_oversized_files(root: &Path) -> Result<Vec<OversizedFileReport>> {
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

pub(crate) fn should_scan_oversized_file(path: &str) -> bool {
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

pub(crate) fn oversized_file_category(path: &str) -> &'static str {
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
    use super::*;

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
