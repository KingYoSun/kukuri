use super::*;
use std::collections::HashMap;
use tempfile::tempdir;

#[derive(Clone, Default)]
struct FakeKeyringStore {
    entries: Arc<Mutex<HashMap<(String, String), String>>>,
    fail_get: Arc<Mutex<bool>>,
    no_default_store: Arc<Mutex<bool>>,
    fail_set: Arc<Mutex<bool>>,
    fail_delete: Arc<Mutex<bool>>,
}

impl KeyringStore for FakeKeyringStore {
    fn get_password(&self, service: &str, account: &str) -> Result<Option<String>> {
        if *self.no_default_store.lock().expect("keyring lock") {
            return Err(anyhow!(KeyringError::NoDefaultStore));
        }
        if *self.fail_get.lock().expect("keyring lock") {
            anyhow::bail!("fake keyring get failure");
        }
        Ok(self
            .entries
            .lock()
            .expect("keyring lock")
            .get(&(service.to_string(), account.to_string()))
            .cloned())
    }

    fn set_password(&self, service: &str, account: &str, secret: &str) -> Result<()> {
        if *self.fail_set.lock().expect("keyring lock") {
            anyhow::bail!("fake keyring set failure");
        }
        self.entries.lock().expect("keyring lock").insert(
            (service.to_string(), account.to_string()),
            secret.to_string(),
        );
        Ok(())
    }

    fn delete_password(&self, service: &str, account: &str) -> Result<()> {
        if *self.no_default_store.lock().expect("keyring lock") {
            return Err(anyhow!(KeyringError::NoDefaultStore));
        }
        if *self.fail_delete.lock().expect("keyring lock") {
            anyhow::bail!("fake keyring delete failure");
        }
        self.entries
            .lock()
            .expect("keyring lock")
            .remove(&(service.to_string(), account.to_string()));
        Ok(())
    }
}

fn clear_identity_env() {
    unsafe { std::env::remove_var("KUKURI_DISABLE_KEYRING") };
}

#[test]
fn auto_mode_prefers_keyring_secret_over_file_secret() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring_secret = KukuriKeys::generate().export_secret_hex();
    let file_secret = KukuriKeys::generate().export_secret_hex();
    let keyring = FakeKeyringStore::default();
    keyring
        .set_password(
            KEYRING_SERVICE,
            keyring_account(&db_path).as_str(),
            keyring_secret.as_str(),
        )
        .expect("seed keyring");
    persist_secret_to_file(&db_path, file_secret.as_str()).expect("seed file");

    let keys = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("load keys");

    assert_eq!(keys.export_secret_hex(), keyring_secret);
    assert_eq!(
        load_backend_marker(&db_path).expect("load backend marker"),
        Some(BACKEND_KEYRING.to_string())
    );
}

#[test]
fn auto_mode_falls_back_to_file_when_keyring_write_fails() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();
    *keyring.fail_set.lock().expect("keyring lock") = true;

    let keys = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("generate keys");

    assert_eq!(
        load_backend_marker(&db_path).expect("load backend marker"),
        Some(BACKEND_FILE.to_string())
    );
    assert_eq!(
        load_secret_from_file(&db_path).expect("load file secret"),
        Some(keys.export_secret_hex())
    );
    assert!(
        keyring
            .get_password(KEYRING_SERVICE, keyring_account(&db_path).as_str())
            .expect("keyring lookup")
            .is_none()
    );
}

#[test]
fn auto_mode_generated_keyring_secret_survives_restart() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();

    let original = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("create keys");
    let restarted = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("reload keys");

    assert_eq!(original.export_secret_hex(), restarted.export_secret_hex());
    assert_eq!(
        load_backend_marker(&db_path).expect("load backend marker"),
        Some(BACKEND_KEYRING.to_string())
    );
}

#[test]
fn existing_keyring_identity_read_failures_preserve_storage_and_recover() {
    for failure in ["get_failure", "no_default_store", "missing_entry"] {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();
        let original =
            load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
                .expect("create keyring identity");
        let marker = std::fs::read(backend_marker_path(&db_path)).expect("backend marker");
        let original_entries = keyring.entries.lock().expect("keyring lock").clone();
        match failure {
            "get_failure" => *keyring.fail_get.lock().expect("keyring lock") = true,
            "no_default_store" => {
                *keyring.no_default_store.lock().expect("keyring lock") = true;
            }
            "missing_entry" => keyring.entries.lock().expect("keyring lock").clear(),
            _ => unreachable!(),
        }
        let inaccessible_entries = keyring.entries.lock().expect("keyring lock").clone();

        assert!(
            load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
                .is_err(),
            "{failure} must not create a replacement identity"
        );
        assert_eq!(
            std::fs::read(backend_marker_path(&db_path)).expect("unchanged marker"),
            marker,
            "{failure}"
        );
        assert_eq!(
            *keyring.entries.lock().expect("keyring lock"),
            inaccessible_entries,
            "{failure} must not mutate keyring entries"
        );
        assert!(!key_file_path(&db_path).exists(), "{failure}");
        assert!(!legacy_key_file_path(&db_path).exists(), "{failure}");

        *keyring.fail_get.lock().expect("keyring lock") = false;
        *keyring.no_default_store.lock().expect("keyring lock") = false;
        *keyring.entries.lock().expect("keyring lock") = original_entries;
        let recovered =
            load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
                .expect("recover original keyring identity");
        assert_eq!(recovered.public_key(), original.public_key(), "{failure}");
        assert!(!key_file_path(&db_path).exists(), "{failure}");
    }
}

#[test]
fn keyring_identity_created_before_db_file_survives_db_creation() {
    // クリーンインストールでは DB ファイル作成前に鍵を keyring へ保存する。
    // DB 作成後に canonicalize の結果(Windows では `\\?\` 付き)が変わっても
    // 同じ entry を引けなければ、marker=keyring のまま起動不能になる。
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();

    let created = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("generate keys before db exists");
    std::fs::write(&db_path, b"").expect("create db file");
    let reloaded = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("reload keys after db exists");

    assert_eq!(reloaded.export_secret_hex(), created.export_secret_hex());
}

#[test]
fn keyring_identity_saved_under_uncanonicalized_account_still_loads() {
    // 修正前の版が DB 不在時に書いた entry(生のパス)を救済できること。
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    std::fs::write(&db_path, b"").expect("create db file");
    let keyring = FakeKeyringStore::default();
    let secret = KukuriKeys::generate().export_secret_hex();
    keyring
        .set_password(
            KEYRING_SERVICE,
            format!("db:{}", db_path.display()).as_str(),
            secret.as_str(),
        )
        .expect("seed legacy keyring entry");
    write_backend_marker(&db_path, BACKEND_KEYRING).expect("seed marker");

    let keys = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
        .expect("load legacy keyring identity");

    assert_eq!(keys.export_secret_hex(), secret);
}

#[test]
fn optional_secret_saved_before_db_file_survives_db_creation() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();

    persist_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        "value",
        &keyring,
    )
    .expect("persist before db exists");
    std::fs::write(&db_path, b"").expect("create db file");

    let loaded = load_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        &keyring,
    )
    .expect("load after db exists");
    assert_eq!(loaded, Some("value".to_string()));
}

#[test]
fn file_only_mode_rejects_existing_keyring_backend_marker() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let secret = KukuriKeys::generate().export_secret_hex();
    let keyring = FakeKeyringStore::default();
    keyring
        .set_password(
            KEYRING_SERVICE,
            keyring_account(&db_path).as_str(),
            secret.as_str(),
        )
        .expect("seed keyring");
    write_backend_marker(&db_path, BACKEND_KEYRING).expect("write backend marker");

    let error = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::FileOnly, &keyring)
        .expect_err("file-only should reject keyring backend");

    assert!(
        error
            .to_string()
            .contains("persisted identity is stored in keyring"),
        "unexpected error: {error}"
    );
}

#[test]
fn optional_secret_keyring_set_failure_does_not_shadow_file_fallback() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();

    persist_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        "old-value",
        &keyring,
    )
    .expect("persist to keyring");
    assert_eq!(
        load_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::Auto,
            "test-purpose",
            "registry",
            &keyring,
        )
        .expect("load from keyring"),
        Some("old-value".to_string())
    );

    // keyring 書き込みが失敗し始めた後の更新(例: registry JSON が blob 上限を超過)。
    // 旧 entry が残ると load(keyring 優先)が file の新しい値を恒久的にシャドウする。
    *keyring.fail_set.lock().expect("keyring lock") = true;
    persist_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        "new-value",
        &keyring,
    )
    .expect("persist with keyring set failure");

    let loaded = load_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        &keyring,
    )
    .expect("load after fallback");
    assert_eq!(
        loaded,
        Some("new-value".to_string()),
        "stale keyring entry must not shadow the file fallback"
    );
}

#[test]
fn optional_secret_uses_file_fallback_without_a_default_keyring() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();
    *keyring.no_default_store.lock().expect("keyring lock") = true;
    persist_secret_to_file_path(
        optional_secret_file_path(&db_path, "test-purpose", "registry").as_path(),
        "file-value",
    )
    .expect("seed file fallback");

    let loaded = load_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        &keyring,
    )
    .expect("missing default keyring must use file fallback");

    assert_eq!(loaded, Some("file-value".to_string()));
}

#[test]
fn optional_secret_keyring_delete_treats_missing_default_store_as_absent() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();
    *keyring.no_default_store.lock().expect("keyring lock") = true;

    delete_optional_secret_keyring_entry_with_keyring(
        &db_path,
        "test-purpose",
        "registry",
        &keyring,
    )
    .expect("missing default keyring is equivalent to an absent entry");
}

#[test]
fn file_only_optional_secret_ignores_keyring_shadow_and_failure() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();
    keyring
        .set_password(
            KEYRING_SERVICE,
            optional_secret_account(&db_path, "test-purpose", "registry").as_str(),
            "stale-keyring-value",
        )
        .expect("seed stale keyring value");
    persist_secret_to_file_path(
        optional_secret_file_path(&db_path, "test-purpose", "registry").as_path(),
        "staged-file-value",
    )
    .expect("seed staged file value");
    *keyring.fail_get.lock().expect("keyring lock") = true;

    let loaded = load_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::FileOnly,
        "test-purpose",
        "registry",
        &keyring,
    )
    .expect("file-only load must not consult keyring");

    assert_eq!(loaded, Some("staged-file-value".to_string()));
}

#[test]
fn optional_secret_persists_to_file_even_when_keyring_delete_fails() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keyring = FakeKeyringStore::default();

    *keyring.fail_set.lock().expect("keyring lock") = true;
    *keyring.fail_delete.lock().expect("keyring lock") = true;
    persist_optional_secret_with_keyring(
        &db_path,
        IdentityStorageMode::Auto,
        "test-purpose",
        "registry",
        "value",
        &keyring,
    )
    .expect("persist must fall back to file even when delete fails");

    assert_eq!(
        load_secret_from_file_path(
            optional_secret_file_path(&db_path, "test-purpose", "registry").as_path()
        )
        .expect("read fallback file"),
        Some("value".to_string())
    );
}

#[cfg(unix)]
#[test]
fn persist_secret_replaces_existing_file_via_rename() {
    use std::os::unix::fs::MetadataExt;

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("kukuri.test-secret");
    persist_secret_to_file_path(&path, "old-value").expect("persist old value");
    let inode_before = std::fs::metadata(&path).expect("metadata before").ino();

    persist_secret_to_file_path(&path, "new-value").expect("persist new value");

    let inode_after = std::fs::metadata(&path).expect("metadata after").ino();
    assert_ne!(
        inode_before, inode_after,
        "persist must replace the file via rename, not truncate it in place"
    );
    assert_eq!(
        load_secret_from_file_path(&path).expect("load after persist"),
        Some("new-value".to_string())
    );
}

#[test]
fn stale_temp_file_does_not_corrupt_persisted_secret() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("kukuri.test-secret");
    persist_secret_to_file_path(&path, "old-value").expect("persist old value");

    // 書き込み途中(rename 前)にクラッシュした状態を再現する。
    let temp_path = path.with_file_name("kukuri.test-secret.tmp");
    std::fs::write(&temp_path, "garbage-from-interrupted-write").expect("write stale temp");

    assert_eq!(
        load_secret_from_file_path(&path).expect("load with stale temp present"),
        Some("old-value".to_string()),
        "interrupted write must leave the previous content readable"
    );

    persist_secret_to_file_path(&path, "new-value").expect("persist over stale temp");
    assert_eq!(
        load_secret_from_file_path(&path).expect("load after recovery"),
        Some("new-value".to_string())
    );
    assert!(
        !temp_path.exists(),
        "persist must consume the temp file via rename"
    );
}

#[cfg(unix)]
#[test]
fn persisted_secret_file_keeps_private_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("kukuri.test-secret");
    persist_secret_to_file_path(&path, "old-value").expect("persist first value");
    persist_secret_to_file_path(&path, "new-value").expect("persist second value");

    let mode = std::fs::metadata(&path)
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "secret file must stay private");
}

#[test]
fn legacy_nsec_file_still_loads() {
    clear_identity_env();
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("kukuri.db");
    let keys = KukuriKeys::generate();
    let legacy_secret = kukuri_core::encode_secret_key_bech32(
        keys.export_secret_hex().as_str(),
        kukuri_core::LEGACY_SECRET_HRP,
    )
    .expect("legacy bech32");
    persist_secret_to_file_path(
        legacy_key_file_path(&db_path).as_path(),
        legacy_secret.as_str(),
    )
    .expect("persist legacy file");

    let restored = load_or_create_keys_with_keyring(
        &db_path,
        IdentityStorageMode::FileOnly,
        &FakeKeyringStore::default(),
    )
    .expect("load legacy keys");

    assert_eq!(restored.export_secret_hex(), keys.export_secret_hex());
}
