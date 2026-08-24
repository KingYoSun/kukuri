use kukuri_cn_protocol::{
    EvidenceReference, EvidenceReferenceKind, RightsCategory, RightsRequestCreateRequest,
    RightsRequesterKind,
};

#[test]
fn rights_request_requires_explicit_scope_acknowledgement_in_the_wire_contract() {
    let request = RightsRequestCreateRequest {
        scope_revision: "scope-v1".to_string(),
        scope_acknowledged: true,
        requester_kind: RightsRequesterKind::RightsHolder,
        requester_name: "権利者".to_string(),
        organization: None,
        address: None,
        email: "rights@example.com".to_string(),
        phone: None,
        represented_rights_holder: None,
        authority_basis: None,
        rights_category: RightsCategory::Copyright,
        rights_basis: "著作権を保有している".to_string(),
        original_work_description: Some("原作品の説明".to_string()),
        original_work_reference: None,
        subject_kind: "post".to_string(),
        subject_id: "post-1".to_string(),
        subject_url: Some("https://node.example/posts/post-1".to_string()),
        infringement_description: "無断で複製されている".to_string(),
        no_permission_statement: true,
        evidence_references: vec![EvidenceReference {
            kind: EvidenceReferenceKind::Url,
            value: "https://rights.example/original".to_string(),
        }],
        requested_capabilities: vec!["community_index".to_string()],
    };

    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["scope_revision"], "scope-v1");
    assert_eq!(value["scope_acknowledged"], true);
    assert_eq!(value["requester_kind"], "rights_holder");
    assert_eq!(value["rights_category"], "copyright");
    assert_eq!(value["evidence_references"][0]["kind"], "url");
    assert_eq!(
        serde_json::from_value::<RightsRequestCreateRequest>(value).unwrap(),
        request
    );
}
