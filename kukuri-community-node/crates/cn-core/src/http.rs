use axum::http::header::HeaderName;
use axum::Router;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;
use tracing::Level;

pub fn apply_standard_layers(router: Router, service_name: &'static str) -> Router {
    let trace = TraceLayer::new_for_http()
        .make_span_with(move |request: &axum::http::Request<_>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("-");
            tracing::info_span!(
                "http.request",
                service = service_name,
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id
            )
        })
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Millis),
        );

    let request_id_header = HeaderName::from_static("x-request-id");

    router
        .layer(trace)
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
}
