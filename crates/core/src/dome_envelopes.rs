use anyhow::{Result, bail};

use crate::game::{
    DomeInstanceManifestV1, DomeMoveRecordV1, DomePresetManifestV1,
    validate_dome_instance_manifest, validate_dome_move_record, validate_dome_preset_manifest,
};

pub fn build_dome_preset_envelope(
    keys: &crate::KukuriKeys,
    manifest: &DomePresetManifestV1,
) -> Result<crate::KukuriEnvelope> {
    if manifest.owner_pubkey != keys.public_key() {
        bail!("Dome preset owner must match signer");
    }
    validate_dome_preset_manifest(manifest)?;
    crate::sign_envelope_json(
        keys,
        "dome-preset",
        vec![
            vec!["author".into(), manifest.owner_pubkey.as_str().into()],
            vec!["object".into(), "dome-preset".into()],
            vec!["preset_id".into(), manifest.preset_id.clone()],
        ],
        manifest,
    )
}

pub fn build_dome_instance_envelope(
    keys: &crate::KukuriKeys,
    manifest: &DomeInstanceManifestV1,
) -> Result<crate::KukuriEnvelope> {
    if manifest.owner_pubkey != keys.public_key() {
        bail!("Dome instance owner must match signer");
    }
    validate_dome_instance_manifest(manifest)?;
    crate::sign_envelope_json(
        keys,
        "dome-instance",
        vec![
            vec!["author".into(), manifest.owner_pubkey.as_str().into()],
            vec!["object".into(), "dome-instance".into()],
            vec!["instance_id".into(), manifest.instance_id.clone()],
            vec!["context".into(), manifest.spatial_context.canonical_id()],
            vec!["generation".into(), manifest.generation.to_string()],
        ],
        manifest,
    )
}

pub fn build_dome_move_envelope(
    keys: &crate::KukuriKeys,
    record: &DomeMoveRecordV1,
) -> Result<crate::KukuriEnvelope> {
    if record.owner_pubkey != keys.public_key() {
        bail!("Dome move owner must match signer");
    }
    validate_dome_move_record(record)?;
    crate::sign_envelope_json(
        keys,
        "dome-move",
        vec![
            vec!["author".into(), record.owner_pubkey.as_str().into()],
            vec!["object".into(), "dome-move".into()],
            vec!["move_id".into(), record.move_id.clone()],
            vec!["phase".into(), format!("{:?}", record.phase).to_lowercase()],
        ],
        record,
    )
}
