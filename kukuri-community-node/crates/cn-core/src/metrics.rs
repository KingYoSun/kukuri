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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn metrics_text_includes_service_name() {
        let body = metrics_text("cn-test");
        assert!(body.contains("# HELP cn_up Service health"));
        assert!(body.contains("cn_up{service=\"cn-test\"} 1"));
    }

    #[test]
    fn metrics_response_sets_content_type() {
        let response = metrics_response("cn-test").into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "text/plain; version=0.0.4");
    }
}
