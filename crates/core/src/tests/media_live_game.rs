use crate::*;

#[test]
fn media_manifest_envelope_uses_protocol_object_kind() {
    let keys = generate_keys();
    let envelope = build_media_manifest_envelope(
        &keys,
        &TopicId::new("kukuri:topic:media"),
        &KukuriMediaManifestV1 {
            manifest_id: "manifest-1".into(),
            owner_pubkey: keys.public_key(),
            created_at: 1,
            items: vec![MediaManifestItem {
                blob_hash: BlobHash::new("blob-1"),
                mime: "image/png".into(),
                size: 123,
                width: Some(10),
                height: Some(10),
                duration_ms: None,
                codec: None,
                thumbnail_blob_hash: None,
            }],
        },
    )
    .expect("manifest envelope");

    envelope.verify().expect("signature verification");
    assert_eq!(envelope.kind, "media-manifest");
}

#[test]
fn fixed_dome_v1_geometry_matches_the_product_contract() {
    let spec = fixed_dome_v1();

    assert_eq!(spec.spec_id, FIXED_DOME_SPEC_ID);
    assert_eq!(spec.inner_radius_cm, 2_000);
    assert_eq!(spec.outer_radius_cm, 2_200);
    assert_eq!(spec.apex_height_cm, 2_000);
    assert_eq!(spec.opening_width_cm, 500);
    assert_eq!(spec.opening_height_cm, 1_000);
    assert_eq!(spec.opening_arch_radius_cm, 250);
    assert_eq!(spec.connection_zone_depth_cm, 1_500);
    assert_eq!(spec.connection_boundary_offset_cm, 2_850);
    assert_eq!(spec.adjacent_dome_center_distance_cm, 5_700);
    assert_eq!(spec.endpoints.len(), 4);
    assert!(spec.physics_enabled);
    assert_eq!(spec.gravity_direction, [0, -1, 0]);
}

#[test]
fn fixed_dome_fallback_capsule_contains_the_bounding_box() {
    let collider =
        fallback_capsule_collider([-100, 0, -50], [100, 400, 50]).expect("valid bounding box");

    assert_eq!(
        collider,
        MetaverseColliderV1::Capsule {
            center: [0, 200, 0],
            radius: 100,
            half_height: 100,
        }
    );
}

#[test]
fn fixed_dome_rejects_the_legacy_scene_payload() {
    let legacy = serde_json::json!({
        "world_version": 1,
        "max_peers": 8,
        "scene": {
            "ground": "default",
            "shared_object": {
                "object_id": "mvp-object-1",
                "asset_ref": null,
                "primitive_fallback": "cube",
                "position": [0, 50, -240],
                "rotation": [0, 0, 0],
                "scale": [100, 100, 100],
                "updated_by": "owner",
                "updated_at": 1
            }
        },
        "default_spawn": { "position": [0, 0, 260], "rotation": [0, 180, 0] },
        "asset_refs": [],
        "chat_history": []
    });

    assert!(serde_json::from_value::<MetaverseRoomStateV1>(legacy).is_err());
}

#[test]
fn fixed_dome_customization_validator_rejects_unsupported_values() {
    let mut customization = DomeCustomizationV1::default();
    customization.environment.gravity_milli = 0;
    assert!(validate_dome_customization(&customization).is_err());

    let mut customization = DomeCustomizationV1::default();
    customization.persistent_props[0].visual_only = true;
    assert!(validate_dome_customization(&customization).is_err());

    let mut customization = DomeCustomizationV1::default();
    customization.persistent_props[0].position = [1_500, 1_500, 0];
    assert!(validate_dome_customization(&customization).is_err());

    let mut customization = DomeCustomizationV1::default();
    customization.surface.wall_texture = Some(MetaverseAssetRef {
        kind: MetaverseAssetKind::Glb,
        blob_hash: "not-a-texture".to_string(),
        mime_type: Some("model/gltf-binary".to_string()),
        size_bytes: Some(1),
        name: Some("mesh.glb".to_string()),
    });
    assert!(validate_dome_customization(&customization).is_err());
}

#[test]
fn fixed_dome_environment_interpolation_is_numeric_and_bounded() {
    let from = DomeEnvironmentV1::default();
    let to = DomeEnvironmentV1 {
        key_light_milli: 4_000,
        ambient_light_milli: 1_000,
        fog_density_micros: 20_000,
        gravity_milli: 4_000,
    };

    assert_eq!(interpolate_dome_environment(&from, &to, 0), from);
    assert_eq!(interpolate_dome_environment(&from, &to, 1_000), to);
    assert_eq!(
        interpolate_dome_environment(&from, &to, 500),
        DomeEnvironmentV1 {
            key_light_milli: 3_200,
            ambient_light_milli: 700,
            fog_density_micros: 14_000,
            gravity_milli: 6_900,
        }
    );
    assert_eq!(interpolate_dome_environment(&from, &to, u16::MAX), to);
}
