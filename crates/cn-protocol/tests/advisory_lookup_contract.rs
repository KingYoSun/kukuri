//! #1056: タイムライン向け content advisory 一括照会の wire contract。

use kukuri_cn_protocol::{
    ADVISORY_LOOKUP_MAX_SUBJECTS, ADVISORY_LOOKUP_PATH, AdvisoryLookupRequest,
    AdvisoryLookupResponse, AdvisorySubjectKind, Basis, ContentAdvisory,
    INVALID_ADVISORY_LOOKUP_CODE, SafetyCategory, is_advisory_blob_hash,
};

#[test]
fn advisory_lookup_path_and_limits_are_stable() {
    assert_eq!(ADVISORY_LOOKUP_PATH, "/v1/advisories/lookup");
    assert_eq!(ADVISORY_LOOKUP_MAX_SUBJECTS, 200);
    assert_eq!(INVALID_ADVISORY_LOOKUP_CODE, "INVALID_ADVISORY_LOOKUP");
}

// INVAR-1: 要求本文は可視の post id と blob hash だけを持つ。
#[test]
fn advisory_lookup_request_carries_only_identifiers() {
    let request = AdvisoryLookupRequest {
        post_ids: vec!["post-1".to_string()],
        blob_hashes: vec!["a".repeat(64)],
    };
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({"post_ids": ["post-1"], "blob_hashes": ["a".repeat(64)]})
    );
    assert_eq!(request.subject_count(), 2);
    let empty: AdvisoryLookupRequest = serde_json::from_str("{}").unwrap();
    assert_eq!(empty.subject_count(), 0);
}

#[test]
fn advisory_lookup_response_wire_shape_is_stable() {
    let response = AdvisoryLookupResponse {
        advisories: vec![ContentAdvisory {
            issuer_node_id: "1".repeat(64),
            subject_kind: AdvisorySubjectKind::BlobCid,
            subject_id: "b".repeat(64),
            category: SafetyCategory::Nsfw,
            label: "adult".to_string(),
            confidence: Some(84),
            signal_id: "signal-1".to_string(),
            basis: Basis::ClassifierScore,
        }],
    };
    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        serde_json::json!({"advisories": [{
            "issuer_node_id": "1".repeat(64),
            "subject_kind": "blob_cid",
            "subject_id": "b".repeat(64),
            "category": "nsfw",
            "label": "adult",
            "confidence": 84,
            "signal_id": "signal-1",
            "basis": "classifier_score"
        }]})
    );
}

#[test]
fn advisory_blob_hash_requires_64_hex_chars() {
    assert!(is_advisory_blob_hash(&"aF09".repeat(16)));
    assert!(!is_advisory_blob_hash(&"a".repeat(63)));
    assert!(!is_advisory_blob_hash(&"g".repeat(64)));
}
