use std::io::Cursor;

use crate::{
    DEVICE_BACKUP_COMPONENT_VERSION, DEVICE_BACKUP_FORMAT_VERSION, DeviceBackupEntryV1,
    DeviceBackupManifestV1, DeviceBackupReader, DeviceBackupWriter, KukuriKeys,
};

fn fixture() -> (DeviceBackupManifestV1, Vec<Vec<u8>>) {
    let keys = KukuriKeys::generate();
    let values = vec![b"sqlite fixture".to_vec(), vec![7u8; 70_000], Vec::new()];
    let entries = ["database", "iroh/blobs/data", "frontend-state"]
        .into_iter()
        .zip(&values)
        .map(|(name, value)| DeviceBackupEntryV1 {
            name: name.to_string(),
            bytes: value.len() as u64,
            blake3: blake3::hash(value).to_hex().to_string(),
        })
        .collect();
    (
        DeviceBackupManifestV1 {
            format_version: DEVICE_BACKUP_FORMAT_VERSION,
            component_version: DEVICE_BACKUP_COMPONENT_VERSION,
            created_at: 1_700_000_000,
            app_version: "0.1.8".to_string(),
            public_key: keys.public_key_hex(),
            account_label: Some("main".to_string()),
            included: vec!["database".to_string(), "local media".to_string()],
            requires_reconsent: vec!["app consent".to_string()],
            entries,
        },
        values,
    )
}

fn archive_bytes(passphrase: &str) -> (DeviceBackupManifestV1, Vec<Vec<u8>>, Vec<u8>) {
    let (manifest, values) = fixture();
    let mut writer = DeviceBackupWriter::new(Vec::new(), passphrase, manifest.clone()).unwrap();
    for value in &values {
        writer.write_entry(Cursor::new(value), |_| Ok(())).unwrap();
    }
    let bytes = writer.finish().unwrap();
    (manifest, values, bytes)
}

#[test]
fn encrypted_backup_round_trips_multiple_chunks_and_empty_entry() {
    let (manifest, values, bytes) = archive_bytes("long enough passphrase");
    let mut reader =
        DeviceBackupReader::open(Cursor::new(bytes), "long enough passphrase").unwrap();
    assert_eq!(reader.manifest(), &manifest);
    for expected in values {
        let mut actual = Vec::new();
        reader.read_entry(&mut actual, |_| Ok(())).unwrap();
        assert_eq!(actual, expected);
    }
    reader.finish().unwrap();
}

#[test]
fn wrong_passphrase_and_corruption_are_rejected() {
    let (_, _, bytes) = archive_bytes("long enough passphrase");
    let err = match DeviceBackupReader::open(Cursor::new(bytes.clone()), "wrong passphrase") {
        Ok(_) => panic!("wrong passphrase unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(err.to_string().contains("wrong passphrase or corrupted"));

    let mut corrupted = bytes;
    let index = corrupted.len() - 8;
    corrupted[index] ^= 0x80;
    let mut reader =
        DeviceBackupReader::open(Cursor::new(corrupted), "long enough passphrase").unwrap();
    let mut failed = false;
    for _ in 0..reader.manifest().entries.len() {
        if reader.read_entry(Vec::new(), |_| Ok(())).is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed);
}

#[test]
fn truncation_and_trailing_data_are_rejected() {
    let (_, values, bytes) = archive_bytes("long enough passphrase");
    let truncated = bytes[..bytes.len() - 6].to_vec();
    let mut reader =
        DeviceBackupReader::open(Cursor::new(truncated), "long enough passphrase").unwrap();
    let mut saw_error = false;
    for _ in values {
        if reader.read_entry(Vec::new(), |_| Ok(())).is_err() {
            saw_error = true;
            break;
        }
    }
    if !saw_error {
        saw_error = reader.finish().is_err();
    }
    assert!(saw_error);

    let (_, values, mut trailing) = archive_bytes("long enough passphrase");
    trailing.push(1);
    let mut reader =
        DeviceBackupReader::open(Cursor::new(trailing), "long enough passphrase").unwrap();
    for _ in values {
        reader.read_entry(Vec::new(), |_| Ok(())).unwrap();
    }
    assert!(reader.finish().is_err());
}

#[test]
fn changed_entry_and_cancel_are_rejected() {
    let (manifest, values) = fixture();
    let mut writer =
        DeviceBackupWriter::new(Vec::new(), "long enough passphrase", manifest.clone()).unwrap();
    let mut changed = values[0].clone();
    changed[0] ^= 1;
    assert!(
        writer
            .write_entry(Cursor::new(changed), |_| Ok(()))
            .unwrap_err()
            .to_string()
            .contains("changed while it was read")
    );

    let mut writer =
        DeviceBackupWriter::new(Vec::new(), "long enough passphrase", manifest).unwrap();
    let err = writer
        .write_entry(Cursor::new(&values[0]), |_| anyhow::bail!("canceled"))
        .unwrap_err();
    assert!(err.to_string().contains("canceled"));
}

#[test]
fn weak_passphrase_and_invalid_manifest_are_rejected() {
    let (mut manifest, _) = fixture();
    let err = match DeviceBackupWriter::new(Vec::new(), "short", manifest.clone()) {
        Ok(_) => panic!("weak passphrase unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(err.to_string().contains("at least"));

    manifest.entries.push(manifest.entries[0].clone());
    let err = match DeviceBackupWriter::new(Vec::new(), "long enough passphrase", manifest) {
        Ok(_) => panic!("duplicate manifest entry unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(err.to_string().contains("duplicate entry name"));
}
