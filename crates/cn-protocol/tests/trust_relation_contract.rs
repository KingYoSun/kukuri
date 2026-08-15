use kukuri_cn_protocol::{
    Proximity, ProximityBasisEntry, RELATION_OPTOUT_PATH, RelationOptoutResponse,
    RelationReadResponse, TrustBasisEntry, TrustComponentKind, TrustReadView,
    TrustUserReadResponse,
};
use kukuri_cn_safety::{
    AppealStatus, Basis, RiskSignalTarget, SafetyCategory, Severity, Visibility,
};

#[test]
fn trust_read_wire_contract_keeps_flattened_view_and_explainable_basis() {
    let response = TrustUserReadResponse {
        viewer_pubkey: "viewer".to_string(),
        view: TrustReadView {
            target_id: "target".to_string(),
            absolute: -0.4,
            relative: -0.2,
            trust: -0.3,
            w_abs_applied: 0.5,
            computed_at: "2026-08-13T00:00:00Z".to_string(),
            basis: vec![TrustBasisEntry {
                signal_id: "signal-1".to_string(),
                issuer_node_id: "node-1".to_string(),
                target: RiskSignalTarget::PostId,
                target_id: "post-1".to_string(),
                component: TrustComponentKind::Relative,
                category: SafetyCategory::Spam,
                severity: Severity::Medium,
                basis: Basis::ClassifierScore,
                confidence: Some(80),
                visibility: Visibility::Local,
                appeal_status: AppealStatus::None,
                expires_at: None,
                raw_contribution: -0.4,
                decay_factor: 0.5,
                relation_weight: 1.0,
                contribution: -0.2,
            }],
        },
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["viewer_pubkey"], "viewer");
    assert_eq!(json["target_id"], "target");
    assert_eq!(json["basis"][0]["component"], "relative");
    assert_eq!(json["basis"][0]["category"], "spam");
    assert_eq!(json["basis"][0]["target"], "post_id");
    assert_eq!(json["basis"][0]["target_id"], "post-1");
    assert_eq!(
        serde_json::from_value::<TrustUserReadResponse>(json).unwrap(),
        response
    );
}

#[test]
fn relation_wire_contract_keeps_flattened_proximity_and_distance_policy() {
    assert_eq!(RELATION_OPTOUT_PATH, "/v1/relation/optout");
    let relation = RelationReadResponse {
        viewer_pubkey: "viewer".to_string(),
        target_pubkey: "target".to_string(),
        proximity: Proximity {
            score: 0.75,
            basis: vec![ProximityBasisEntry {
                feature: "shared_topics".to_string(),
                value: 3.0,
                weight: 1.0,
                contribution: 0.75,
            }],
        },
    };
    let json = serde_json::to_value(&relation).unwrap();
    assert_eq!(json["score"], 0.75);
    assert!(json.get("proximity").is_none());

    let optout = RelationOptoutResponse {
        pubkey: "viewer".to_string(),
        opted_out: true,
        opted_out_at: Some("2026-08-13T00:00:00Z".to_string()),
        min_proximity: 0.25,
    };
    let optout_json = serde_json::to_value(optout).unwrap();
    assert_eq!(
        optout_json,
        serde_json::json!({
            "pubkey": "viewer",
            "opted_out": true,
            "opted_out_at": "2026-08-13T00:00:00Z",
            "min_proximity": 0.25
        })
    );
}
