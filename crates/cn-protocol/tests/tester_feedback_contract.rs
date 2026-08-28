//! テスターフィードバック(#802 / ADR 0039)の wire 契約。

use kukuri_cn_protocol::{
    CommunityNodeTesterFeedbackRequest, CommunityNodeTesterFeedbackResponse,
    TESTER_FEEDBACK_MAX_CHARS,
};

#[test]
fn tester_feedback_request_roundtrips_all_fields() {
    let request = CommunityNodeTesterFeedbackRequest {
        what_attempted: "投稿を作成しようとした".to_string(),
        what_happened: "送信ボタンを押しても反応がなかった".to_string(),
        what_seemed_wrong: "エラーも成功も表示されないのが変だと思った".to_string(),
        client_version: "0.1.7".to_string(),
        os: "linux".to_string(),
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["what_attempted"], "投稿を作成しようとした");
    assert_eq!(value["what_happened"], "送信ボタンを押しても反応がなかった");
    assert_eq!(
        value["what_seemed_wrong"],
        "エラーも成功も表示されないのが変だと思った"
    );
    assert_eq!(value["client_version"], "0.1.7");
    assert_eq!(value["os"], "linux");
    assert_eq!(
        serde_json::from_value::<CommunityNodeTesterFeedbackRequest>(value).unwrap(),
        request
    );
}

#[test]
fn tester_feedback_request_defaults_missing_fields_and_ignores_unknown() {
    let request: CommunityNodeTesterFeedbackRequest = serde_json::from_value(serde_json::json!({
        "what_attempted": "設定を開こうとした",
        "unknown_future_field": true,
    }))
    .unwrap();
    assert_eq!(request.what_attempted, "設定を開こうとした");
    assert_eq!(request.what_happened, "");
    assert_eq!(request.what_seemed_wrong, "");
    assert_eq!(request.client_version, "");
    assert_eq!(request.os, "");
}

#[test]
fn tester_feedback_response_roundtrips_and_omits_absent_reference() {
    let response = CommunityNodeTesterFeedbackResponse {
        reference_id: Some("feedback-1".to_string()),
    };
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["reference_id"], "feedback-1");
    assert_eq!(
        serde_json::from_value::<CommunityNodeTesterFeedbackResponse>(value).unwrap(),
        response
    );

    let empty = CommunityNodeTesterFeedbackResponse { reference_id: None };
    let value = serde_json::to_value(&empty).unwrap();
    assert!(value.get("reference_id").is_none());
    assert_eq!(
        serde_json::from_value::<CommunityNodeTesterFeedbackResponse>(serde_json::json!({}))
            .unwrap(),
        empty
    );
}

#[test]
fn tester_feedback_max_chars_is_two_thousand_codepoints() {
    assert_eq!(TESTER_FEEDBACK_MAX_CHARS, 2000);
}
