use super::trending_metrics_metrics::TrendingMetricsRecorder;
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server, StatusCode};

const CONTENT_TYPE: &str = "text/plain; version=0.0.4";

pub fn spawn_prometheus_exporter(port: u16, recorder: Arc<TrendingMetricsRecorder>) {
    let address = format!("127.0.0.1:{port}");
    tauri::async_runtime::spawn_blocking(move || match Server::http(&address) {
        Ok(server) => {
            tracing::info!(
                target: "metrics::trending",
                port,
                "prometheus exporter listening"
            );
            for request in server.incoming_requests() {
                let response = if request.method() == &Method::Get && request.url() == "/metrics" {
                    match recorder.encode() {
                        Ok(body) => Response::from_data(body)
                            .with_status_code(StatusCode(200))
                            .with_header(Header::from_bytes("Content-Type", CONTENT_TYPE).unwrap()),
                        Err(err) => {
                            tracing::error!(
                                target: "metrics::trending",
                                error = %err,
                                "failed to encode metrics payload"
                            );
                            Response::from_string("failed to encode metrics")
                                .with_status_code(StatusCode(500))
                        }
                    }
                } else {
                    Response::from_string("not found").with_status_code(StatusCode(404))
                };

                if let Err(err) = request.respond(response) {
                    tracing::warn!(
                        target: "metrics::trending",
                        error = %err,
                        "failed to respond to metrics request"
                    );
                }
            }
        }
        Err(err) => {
            tracing::error!(
                target: "metrics::trending",
                port,
                error = %err,
                "failed to bind prometheus exporter"
            );
        }
    });
}
