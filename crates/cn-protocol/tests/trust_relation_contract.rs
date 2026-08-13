use kukuri_cn_protocol::{RELATION_OPTOUT_PATH, RelationOptoutResponse};

#[test]
fn relation_optout_status_wire_contract_is_stable() {
    assert_eq!(RELATION_OPTOUT_PATH, "/v1/relation/optout");
    let response = RelationOptoutResponse {
        pubkey: "aa".repeat(32),
        opted_out: true,
        opted_out_at: Some("2026-08-13T00:00:00Z".to_string()),
        min_proximity: 0.5,
    };
    assert_eq!(
        serde_json::to_value(&response).expect("serialize relation opt-out status"),
        serde_json::json!({
            "pubkey": "aa".repeat(32),
            "opted_out": true,
            "opted_out_at": "2026-08-13T00:00:00Z",
            "min_proximity": 0.5
        })
    );
    let decoded: RelationOptoutResponse = serde_json::from_value(serde_json::json!({
        "pubkey": "bb".repeat(32),
        "opted_out": false,
        "opted_out_at": null,
        "min_proximity": 0.25
    }))
    .expect("deserialize relation opt-out status");
    assert!(!decoded.opted_out);
    assert_eq!(decoded.min_proximity, 0.25);
}
