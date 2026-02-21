use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn openapi_contract_contains_user_paths() {
    let app = Router::new().route("/v1/openapi.json", get(crate::openapi_json));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/openapi.json")
        .header("host", "localhost:8080")
        .body(Body::empty())
        .expect("request");
    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let payload: Value = serde_json::from_slice(&body).expect("json body");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload.get("openapi").and_then(Value::as_str),
        Some("3.1.0")
    );
    assert!(payload
        .pointer("/paths/~1v1~1auth~1challenge/post")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1bootstrap~1nodes/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1bootstrap~1hints~1latest/get")
        .is_some());
    assert_eq!(
        payload
            .pointer("/paths/~1v1~1bootstrap~1hints~1latest/get/parameters/0/name")
            .and_then(Value::as_str),
        Some("since")
    );
    assert_eq!(
        payload
            .pointer(
                "/paths/~1v1~1bootstrap~1hints~1latest/get/responses/200/content/application~1json/schema/$ref"
            )
            .and_then(Value::as_str),
        Some("#/components/schemas/BootstrapHintLatestResponse")
    );
    assert!(payload
        .pointer("/paths/~1v1~1bootstrap~1hints~1latest/get/responses/204")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1bootstrap~1hints~1latest/get/responses/401")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1bootstrap~1hints~1latest/get/responses/428")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1bootstrap~1hints~1latest/get/responses/429")
        .is_some());
    assert!(payload
        .pointer("/components/schemas/BootstrapHintLatestResponse/properties/seq")
        .is_some());
    assert!(payload
        .pointer("/components/schemas/BootstrapHintLatestResponse/properties/received_at")
        .is_some());
    assert!(payload
        .pointer("/components/schemas/BootstrapHintLatestResponse/properties/hint")
        .is_some());
    assert!(payload.pointer("/paths/~1v1~1search/get").is_some());
    assert!(payload
        .pointer("/paths/~1v1~1communities~1suggest/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1trust~1report-based/get")
        .is_some());
    assert!(payload
        .pointer("/paths/~1v1~1trust~1communication-density/get")
        .is_some());
    assert_eq!(
        payload
            .pointer("/paths/~1v1~1trust~1report-based/get/parameters/0/name")
            .and_then(Value::as_str),
        Some("subject")
    );
    assert!(payload
        .pointer("/paths/~1v1~1trust~1report-based/get/parameters/0/description")
        .and_then(Value::as_str)
        .map(|value| value.contains("addressable:"))
        .unwrap_or(false));
    assert!(payload
        .pointer("/paths/~1v1~1topic-subscription-requests/post/responses/429")
        .is_some());
    assert_eq!(
        payload
            .pointer(
                "/paths/~1v1~1topic-subscription-requests/post/responses/429/content/application~1json/schema/$ref"
            )
            .and_then(Value::as_str),
        Some("#/components/schemas/ErrorResponse")
    );
    assert!(payload
        .pointer("/paths/~1v1~1personal-data-deletion-requests/post")
        .is_some());
}
