use crate::{
    MetaverseAssetKind, MetaverseBudgetResource, MetaverseBudgetScope,
    MetaverseResourceBudgetConfig, MetaverseResourceRejectionReason, inspect_metaverse_asset,
};

#[test]
fn default_budget_preserves_existing_metaverse_limits() {
    let budget = MetaverseResourceBudgetConfig::default();
    assert_eq!(budget.dome.max_persistent_props, 64);
    assert_eq!(budget.host.max_participants, 64);
    assert_eq!(budget.dome.max_snapshot_hz, 10);
    assert_eq!(budget.client.cache_capacity_bytes, 1024 * 1024 * 1024);
    assert_eq!(budget.player.max_audio_frames_per_second, 50);
    assert_eq!(budget.client.max_concurrent_audio_streams, 16);
    budget.validate().expect("default budget is valid");
}

#[test]
fn budget_configuration_rejects_zero_and_unsafe_values() {
    let mut budget = MetaverseResourceBudgetConfig::default();
    budget.player.max_impulse_centimeters = 0;
    assert!(budget.validate().is_err());

    let mut budget = MetaverseResourceBudgetConfig::default();
    budget.dome.max_snapshot_hz = 61;
    assert!(budget.validate().is_err());

    let mut budget = MetaverseResourceBudgetConfig::default();
    budget.host.max_participants = 513;
    assert!(budget.validate().is_err());
}

#[test]
fn budget_configuration_accepts_a_stricter_operational_profile() {
    let mut budget = MetaverseResourceBudgetConfig::default();
    budget.dome.max_persistent_props = 8;
    budget.host.max_participants = 12;
    budget.client.cache_capacity_bytes = 256 * 1024 * 1024;
    let json = serde_json::to_string(&budget).expect("serialize budget");

    assert_eq!(
        MetaverseResourceBudgetConfig::from_json_override(Some(&json))
            .expect("stricter profile is valid"),
        budget
    );
}

#[test]
fn resource_rejection_has_stable_machine_readable_fields() {
    let rejection = crate::MetaverseResourceRejection::new(
        MetaverseBudgetScope::Player,
        MetaverseBudgetResource::GuestProps,
        MetaverseResourceRejectionReason::LimitExceeded,
        17,
        16,
    );
    assert_eq!(
        rejection.code(),
        "METAVERSE_PLAYER_GUEST_PROPS_LIMIT_EXCEEDED"
    );
    assert_eq!(rejection.observed, 17);
    assert_eq!(rejection.limit, 16);
}

#[test]
fn texture_inspection_reads_dimensions_from_bytes() {
    let png = [
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0,
        0, 2, 0, 0, 0, 3,
    ];
    let metadata = inspect_metaverse_asset(MetaverseAssetKind::Texture, &png)
        .expect("PNG header is inspectable");
    assert_eq!(metadata.texture_width, Some(2));
    assert_eq!(metadata.texture_height, Some(3));
    assert_eq!(metadata.decoded_texture_bytes, 24);
}

#[test]
fn glb_inspection_counts_triangles_from_accessors() {
    let json = br#"{"asset":{"version":"2.0"},"accessors":[{"count":6}],"meshes":[{"primitives":[{"indices":0,"mode":4}]}]}"#;
    let padded_len = (json.len() + 3) & !3;
    let total_len = 12 + 8 + padded_len;
    let mut glb = Vec::with_capacity(total_len);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());
    glb.extend_from_slice(&(padded_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f534a_u32.to_le_bytes());
    glb.extend_from_slice(json);
    glb.resize(total_len, b' ');

    let metadata =
        inspect_metaverse_asset(MetaverseAssetKind::Glb, &glb).expect("GLB is inspectable");
    assert_eq!(metadata.model_triangles, 2);
}

#[test]
fn asset_inspection_rejects_oversized_texture_and_model_bomb() {
    let oversized_png = [
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0,
        0x40, 0x01, 0, 0, 0, 1,
    ];
    assert!(inspect_metaverse_asset(MetaverseAssetKind::Texture, &oversized_png).is_err());

    let json = br#"{"asset":{"version":"2.0"},"accessors":[{"count":60000003}],"meshes":[{"primitives":[{"indices":0,"mode":4}]}]}"#;
    let padded_len = (json.len() + 3) & !3;
    let total_len = 12 + 8 + padded_len;
    let mut glb = Vec::with_capacity(total_len);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());
    glb.extend_from_slice(&(padded_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f534a_u32.to_le_bytes());
    glb.extend_from_slice(json);
    glb.resize(total_len, b' ');
    assert!(inspect_metaverse_asset(MetaverseAssetKind::Glb, &glb).is_err());
}

#[test]
fn glb_inspection_accounts_for_embedded_texture_memory() {
    let json = br#"{"asset":{"version":"2.0"},"accessors":[{"count":3}],"meshes":[{"primitives":[{"indices":0}]}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":24}],"images":[{"bufferView":0,"mimeType":"image/png"}]}"#;
    let padded_json_len = (json.len() + 3) & !3;
    let png = [
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0,
        0, 2, 0, 0, 0, 3,
    ];
    let total_len = 12 + 8 + padded_json_len + 8 + png.len();
    let mut glb = Vec::with_capacity(total_len);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());
    glb.extend_from_slice(&(padded_json_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f534a_u32.to_le_bytes());
    glb.extend_from_slice(json);
    glb.resize(12 + 8 + padded_json_len, b' ');
    glb.extend_from_slice(&(png.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e4942_u32.to_le_bytes());
    glb.extend_from_slice(&png);

    let metadata =
        inspect_metaverse_asset(MetaverseAssetKind::Glb, &glb).expect("embedded PNG is inspected");
    assert_eq!(metadata.texture_width, Some(2));
    assert_eq!(metadata.texture_height, Some(3));
    assert_eq!(metadata.decoded_texture_bytes, 24);
}
