use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;

pub fn metrics_text(service_name: &str) -> String {
    format!(
        "# HELP cn_up Service health
# TYPE cn_up gauge
cn_up{{service=\"{}\"}} 1
",
        service_name
    )
}

pub fn metrics_response(service_name: &str) -> impl IntoResponse {
    let body = metrics_text(service_name);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "text/plain; version=0.0.4".parse().unwrap(),
    );
    (StatusCode::OK, headers, body)
}
