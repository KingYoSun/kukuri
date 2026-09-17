//! CI の Cache Volume から workspace crate の build 成果物を除く（#1120）。
//!
//! CI は checkout で source の mtime が更新されるため、cache が当たっても workspace crate は
//! 毎回再 compile される。その成果物を volume に残しても再利用されず、容量だけを使う。
//! 依存 crate の成果物は残し、workspace crate（root workspace と `apps/desktop/src-tauri`）の
//! 成果物だけを `deps` / `.fingerprint` / `build` / `incremental` と profile 直下から除く。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// cargo が profile directory の下に作る、crate ごとの成果物置き場。
const ARTIFACT_DIRS: [&str; 4] = ["deps", ".fingerprint", "build", "incremental"];
/// target root から profile directory までの最大の深さ（`<root>/<triple>/<profile>` と
/// `<root>/desktop-tauri-check/<profile>` を含む）。
const MAX_PROFILE_DEPTH: usize = 3;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PruneStats {
    pub(crate) entries: usize,
    pub(crate) bytes: u64,
}

pub(crate) fn ci_prune_target() -> Result<()> {
    let root = crate::root_dir();
    let tauri_manifest = root.join("apps/desktop/src-tauri/Cargo.toml");
    let mut names = workspace_artifact_names(&cargo_metadata(&root, None)?)?;
    // src-tauri は root workspace の外にある。親 directory に別の workspace がある環境
    // （repo 内の git worktree など）では metadata を取れないため、削除は root workspace
    // の分だけにして続ける。成果物の削除は cache の容量対策であり、CI を止めない。
    match cargo_metadata(&root, Some(&tauri_manifest))
        .and_then(|metadata| workspace_artifact_names(&metadata))
    {
        Ok(tauri_names) => names.extend(tauri_names),
        Err(error) => eprintln!(
            "[xtask] warning: ci-prune-target skips apps/desktop/src-tauri packages: {error:#}"
        ),
    }
    // 実行中の xtask 自身は Windows では削除できないため対象外にする。
    names.remove("xtask");

    let mut total = PruneStats::default();
    for target_root in [
        root.join("target"),
        root.join("apps/desktop/src-tauri/target"),
    ] {
        let stats = prune_target_root(&target_root, &names)?;
        println!(
            "[xtask] ci-prune-target {}: removed {} entries ({:.1} MiB)",
            target_root.display(),
            stats.entries,
            stats.bytes as f64 / (1024.0 * 1024.0)
        );
        total.entries += stats.entries;
        total.bytes += stats.bytes;
    }
    println!(
        "[xtask] ci-prune-target total: removed {} entries ({:.1} MiB)",
        total.entries,
        total.bytes as f64 / (1024.0 * 1024.0)
    );
    Ok(())
}

fn cargo_metadata(root: &Path, manifest: Option<&Path>) -> Result<Value> {
    let mut command = Command::new("cargo");
    command.args(["metadata", "--no-deps", "--format-version", "1"]);
    if let Some(manifest) = manifest {
        command.arg("--manifest-path").arg(manifest);
    }
    let output = command
        .current_dir(root)
        .output()
        .context("failed to execute cargo metadata for ci-prune-target")?;
    if !output.status.success() {
        bail!(
            "cargo metadata for ci-prune-target failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata output")
}

/// workspace の package 名と target 名を、成果物のファイル名と同じ正規化（`-` → `_`）で集める。
pub(crate) fn workspace_artifact_names(metadata: &Value) -> Result<BTreeSet<String>> {
    let packages = metadata["packages"]
        .as_array()
        .context("cargo metadata is missing the packages array")?;
    let mut names = BTreeSet::new();
    for package in packages {
        let name = package["name"]
            .as_str()
            .context("cargo metadata package is missing a string name")?;
        names.insert(normalize(name));
        for target in package["targets"].as_array().into_iter().flatten() {
            if let Some(target_name) = target["name"].as_str() {
                names.insert(normalize(target_name));
            }
        }
    }
    Ok(names)
}

fn normalize(name: &str) -> String {
    name.replace('-', "_")
}

/// `deps` などの entry 名から crate 名を取り出す。cargo の metadata hash（16 桁の 16 進数）が
/// 付いていない名前は対象にしない。
fn hashed_entry_crate_name(entry_name: &str) -> Option<String> {
    let stem = entry_name.split('.').next()?;
    let (name, hash) = stem.rsplit_once('-')?;
    if hash.len() != 16 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) || name.is_empty() {
        return None;
    }
    Some(normalize(name))
}

fn matches_workspace(name: &str, names: &BTreeSet<String>) -> bool {
    names.contains(name)
        || name
            .strip_prefix("lib")
            .is_some_and(|rest| names.contains(rest))
}

pub(crate) fn prune_target_root(root: &Path, names: &BTreeSet<String>) -> Result<PruneStats> {
    let mut stats = PruneStats::default();
    if !root.is_dir() {
        return Ok(stats);
    }
    for profile_dir in profile_dirs(root)? {
        for artifact_dir in ARTIFACT_DIRS {
            let dir = profile_dir.join(artifact_dir);
            if !dir.is_dir() {
                continue;
            }
            for entry in read_dir_sorted(&dir)? {
                let file_name = entry
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                if hashed_entry_crate_name(&file_name)
                    .is_some_and(|name| matches_workspace(&name, names))
                {
                    remove_entry(&entry, &mut stats)?;
                }
            }
        }
        // profile 直下には、指定した workspace target の成果物だけが hash なしで置かれる。
        for entry in read_dir_sorted(&profile_dir)? {
            if !entry.is_file() {
                continue;
            }
            let file_name = entry
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let stem = normalize(file_name.split('.').next().unwrap_or_default());
            if !stem.is_empty() && matches_workspace(&stem, names) {
                remove_entry(&entry, &mut stats)?;
            }
        }
    }
    Ok(stats)
}

/// `deps` か `.fingerprint` を直下に持つ directory を profile directory とみなす。
fn profile_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if dir.join("deps").is_dir() || dir.join(".fingerprint").is_dir() {
            found.push(dir);
            continue;
        }
        if depth >= MAX_PROFILE_DEPTH {
            continue;
        }
        for entry in read_dir_sorted(&dir)? {
            if entry.is_dir() && !entry.is_symlink() {
                stack.push((entry, depth + 1));
            }
        }
    }
    found.sort();
    Ok(found)
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to list {}", dir.display()))?;
    entries.sort();
    Ok(entries)
}

fn remove_entry(path: &Path, stats: &mut PruneStats) -> Result<()> {
    let meta =
        fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    if meta.is_dir() {
        stats.bytes += dir_size(path)?;
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    } else {
        stats.bytes += meta.len();
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    stats.entries += 1;
    Ok(())
}

fn dir_size(dir: &Path) -> Result<u64> {
    let mut total = 0;
    for entry in read_dir_sorted(dir)? {
        let meta = fs::symlink_metadata(&entry)
            .with_context(|| format!("failed to stat {}", entry.display()))?;
        total += if meta.is_dir() {
            dir_size(&entry)?
        } else {
            meta.len()
        };
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const HASH: &str = "0123456789abcdef";

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "kukuri-xtask-ci-prune-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, rel: &str) -> PathBuf {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"artifact").unwrap();
            path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn names() -> BTreeSet<String> {
        workspace_artifact_names(&json!({
            "packages": [
                {"name": "kukuri-core", "targets": [{"name": "kukuri_core"}]},
                {"name": "kukuri-desktop-tauri", "targets": [
                    {"name": "kukuri_desktop_tauri_lib"}, {"name": "kukuri-desktop-tauri"}, {"name": "build-script-build"}
                ]},
                {"name": "xtask", "targets": [{"name": "xtask"}]}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn collects_package_and_target_names_normalized() {
        let names = names();
        for expected in [
            "kukuri_core",
            "kukuri_desktop_tauri",
            "kukuri_desktop_tauri_lib",
            "xtask",
        ] {
            assert!(
                names.contains(expected),
                "{expected} missing from {names:?}"
            );
        }
    }

    #[test]
    fn only_hashed_entries_yield_crate_names() {
        assert_eq!(
            hashed_entry_crate_name(&format!("libkukuri_core-{HASH}.rlib")).as_deref(),
            Some("libkukuri_core")
        );
        assert_eq!(
            hashed_entry_crate_name(&format!("kukuri-core-{HASH}")).as_deref(),
            Some("kukuri_core")
        );
        assert_eq!(hashed_entry_crate_name("kukuri_core.d"), None);
        assert_eq!(hashed_entry_crate_name("serde-1.0"), None);
        assert_eq!(hashed_entry_crate_name(&format!("-{HASH}")), None);
    }

    #[test]
    fn removes_workspace_artifacts_and_keeps_dependencies() {
        let tree = TempTree::new("mixed");
        let names = names();
        let removed = [
            format!("debug/deps/libkukuri_core-{HASH}.rlib"),
            format!("debug/deps/libkukuri_core-{HASH}.rmeta"),
            format!("debug/deps/kukuri_core-{HASH}.d"),
            format!("debug/deps/kukuri_core-{HASH}"),
            format!("debug/.fingerprint/kukuri-core-{HASH}/lib-kukuri_core"),
            format!("debug/build/kukuri-desktop-tauri-{HASH}/output"),
            format!("debug/incremental/kukuri_core-{HASH}/s-abc/query-cache.bin"),
            format!("desktop-tauri-check/debug/deps/libkukuri_desktop_tauri_lib-{HASH}.rlib"),
            format!("x86_64-pc-windows-msvc/release/deps/kukuri_desktop_tauri-{HASH}.exe"),
            format!("x86_64-pc-windows-msvc/release/deps/kukuri_desktop_tauri-{HASH}.pdb"),
            "debug/xtask".to_string(),
            "debug/xtask.d".to_string(),
            "debug/libkukuri_core.rlib".to_string(),
            "x86_64-pc-windows-msvc/release/kukuri-desktop-tauri.exe".to_string(),
        ];
        let kept = [
            format!("debug/deps/libserde-{HASH}.rlib"),
            format!("debug/deps/serde-{HASH}.d"),
            format!("debug/deps/libkukuri_core_extra-{HASH}.rlib"),
            format!("debug/.fingerprint/serde-{HASH}/lib-serde"),
            format!("debug/build/ring-{HASH}/output"),
            format!("debug/deps/liblibc-{HASH}.rlib"),
            "debug/.cargo-lock".to_string(),
            "x86_64-pc-windows-msvc/release/bundle/nsis/kukuri.exe".to_string(),
            "CACHEDIR.TAG".to_string(),
        ];
        let removed_paths: Vec<_> = removed.iter().map(|rel| tree.file(rel)).collect();
        let kept_paths: Vec<_> = kept.iter().map(|rel| tree.file(rel)).collect();

        let stats = prune_target_root(&tree.0, &names).unwrap();

        for path in &removed_paths {
            assert!(!path.exists(), "{} should be removed", path.display());
        }
        for path in &kept_paths {
            assert!(path.exists(), "{} should be kept", path.display());
        }
        // `.fingerprint` / `build` / `incremental` の directory は 1 entry として数える。
        assert_eq!(stats.entries, removed.len());
        assert_eq!(
            stats.bytes,
            (removed.len() as u64) * b"artifact".len() as u64
        );
    }

    #[test]
    fn missing_target_root_is_a_no_op() {
        let tree = TempTree::new("missing");
        let stats = prune_target_root(&tree.0.join("target"), &names()).unwrap();
        assert_eq!(stats, PruneStats::default());
    }
}
