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
    assert_eq!(payload.get("openapi").and_then(Value::as_str), Some("3.0.3"));
    assert!(payload.pointer("/paths/~1v1~1auth~1challenge/post").is_some());
    assert!(payload.pointer("/paths/~1v1~1bootstrap~1nodes/get").is_some());
    assert!(payload.pointer("/paths/~1v1~1search/get").is_some());
    assert!(
        payload
            .pointer("/paths/~1v1~1personal-data-deletion-requests/post")
            .is_some()
    );
}
