//! WP-Q7 T5 の lock 分類 contract。
//!
//! desktop-runtime のテストが取得する共有資源 lock(kukuri-test-support の
//! `TestResource`)の全取得箇所を、ファイル別・資源別の分類表として固定する。
//! 本表に載らないファイルは「lock 不要」分類を意味し、取得 0 件であることを
//! 同時に検証する。
//!
//! テストの lock 取得を追加・削除・移動した場合は、本表を更新して分類を
//! 宣言すること(未分類のままの追加はこのテストが fail する)。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// (tests/ からの相対パス, TestResource variant 名, 取得数)。
const LOCK_CLASSIFICATION: &[(&str, &str, usize)] = &[
    ("community_node/config.rs", "ProcessEnvironment", 3),
    ("community_node/connectivity.rs", "IrohNetwork", 6),
    ("community_node/metadata.rs", "CommunityNodeServer", 9),
    ("community_node/scheduler.rs", "CommunityNodeServer", 3),
    ("community_node/session.rs", "CommunityNodeServer", 6),
    ("identity_restart.rs", "IdentityStorage", 2),
    ("media_blob_restore.rs", "IrohNetwork", 11),
    ("private_channels/friend_only.rs", "IrohNetwork", 1),
    ("private_channels/friend_plus.rs", "IrohNetwork", 1),
    ("private_channels/invite.rs", "IrohNetwork", 3),
    ("private_channels/persistence.rs", "IrohNetwork", 2),
    ("runtime_events.rs", "CommunityNodeServer", 1),
    ("seeded_dht.rs", "DhtTestnet", 3),
    ("static_peer.rs", "IrohNetwork", 2),
];

/// 取得呼び出しの検出パターン。本ファイル自身がマッチしないよう分割して組み立てる。
fn acquisition_pattern() -> String {
    format!("{}({}::", "lock_test_resource", "TestResource")
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn scan_acquisitions(tests_root: &Path) -> BTreeMap<(String, String), usize> {
    let pattern = acquisition_pattern();
    let mut files = Vec::new();
    collect_rs_files(tests_root, &mut files);
    let mut acquisitions: BTreeMap<(String, String), usize> = BTreeMap::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let relative = path
            .strip_prefix(tests_root)
            .expect("path under tests root")
            .to_string_lossy()
            .replace('\\', "/");
        for (offset, _) in source.match_indices(&pattern) {
            let variant_start = offset + pattern.len();
            let variant_end = source[variant_start..]
                .find(')')
                .map(|end| variant_start + end)
                .unwrap_or_else(|| panic!("unterminated acquisition expression in {relative}"));
            let variant = source[variant_start..variant_end].trim().to_string();
            *acquisitions.entry((relative.clone(), variant)).or_default() += 1;
        }
    }
    acquisitions
}

/// Q7 T5 の受入条件「全取得箇所が分類され、未分類追加で test が fail」を固定する。
#[test]
fn lock_acquisitions_match_declared_classification() {
    let tests_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests");
    let actual = scan_acquisitions(&tests_root);
    let expected: BTreeMap<(String, String), usize> = LOCK_CLASSIFICATION
        .iter()
        .map(|(file, resource, count)| (((*file).to_string(), (*resource).to_string()), *count))
        .collect();
    assert_eq!(
        actual, expected,
        "lock acquisitions drifted from the declared classification. \
         テストの lock 取得を追加・削除・移動した場合は \
         support/lock_contract.rs の LOCK_CLASSIFICATION を更新して分類を宣言すること \
         (表に無いファイルは lock 不要分類 = 取得 0 件を要求する)"
    );
    let total: usize = expected.values().sum();
    assert_eq!(
        total, 53,
        "classification total drifted from the Q7 T6 baseline"
    );
}
