use kukuri_cn_protocol::{
    CommunityNodeReportAppeal, CommunityNodeReportRequest, CommunityNodeReportResponse,
};

#[test]
fn report_request_roundtrips_appeal_and_omits_absent_contact() {
    let request = CommunityNodeReportRequest {
        subject_kind: "post".to_string(),
        subject_id: "post-1".to_string(),
        capability: "moderation".to_string(),
        reason: "other".to_string(),
        details: Some("誤判定として再確認を求めます".to_string()),
        reporter_contact: None,
        appeal: Some(CommunityNodeReportAppeal {
            risk_signal_id: "signal-1".to_string(),
        }),
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["subject_kind"], "post");
    assert_eq!(value["appeal"]["risk_signal_id"], "signal-1");
    assert!(value.get("reporter_contact").is_none());
    assert_eq!(
        serde_json::from_value::<CommunityNodeReportRequest>(value).unwrap(),
        request
    );
}

#[test]
fn report_response_roundtrips_disputed_signal() {
    let response = CommunityNodeReportResponse {
        reference_id: Some("report-1".to_string()),
        disputed_risk_signal_id: Some("signal-1".to_string()),
    };
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["reference_id"], "report-1");
    assert_eq!(value["disputed_risk_signal_id"], "signal-1");
    assert_eq!(
        serde_json::from_value::<CommunityNodeReportResponse>(value).unwrap(),
        response
    );
}
