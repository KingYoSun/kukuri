use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

struct Metrics {
    registry: Registry,
    cn_up: IntGaugeVec,
    http_requests_total: IntCounterVec,
    http_request_duration_seconds: HistogramVec,
    ws_connections: IntGaugeVec,
    ws_unauthenticated_connections: IntGaugeVec,
    ws_req_total: IntCounterVec,
    ws_event_total: IntCounterVec,
    ws_auth_disconnect_total: IntCounterVec,
    ingest_received_total: IntCounterVec,
    ingest_rejected_total: IntCounterVec,
    gossip_received_total: IntCounterVec,
    gossip_sent_total: IntCounterVec,
    bootstrap_hint_publish_total: IntCounterVec,
    dedupe_hits_total: IntCounterVec,
    dedupe_misses_total: IntCounterVec,
    auth_success_total: IntCounterVec,
    auth_failure_total: IntCounterVec,
    consent_required_total: IntCounterVec,
    quota_exceeded_total: IntCounterVec,
    outbox_backlog: IntGaugeVec,
    outbox_consumer_batches_total: IntCounterVec,
    outbox_consumer_processing_duration_seconds: HistogramVec,
    outbox_consumer_batch_size: HistogramVec,
    suggest_stage_a_latency_ms: HistogramVec,
    suggest_stage_b_latency_ms: HistogramVec,
    suggest_block_filter_drop_count: IntCounterVec,
    backfill_processed_rows: IntGaugeVec,
    backfill_eta_seconds: IntGaugeVec,
    shadow_overlap_at_10: HistogramVec,
    shadow_latency_delta_ms: HistogramVec,
    search_dual_write_errors_total: IntCounterVec,
    search_dual_write_retries_total: IntCounterVec,
}

pub const OUTBOX_CONSUMER_RESULT_SUCCESS: &str = "success";
pub const OUTBOX_CONSUMER_RESULT_ERROR: &str = "error";

static METRICS: OnceLock<Metrics> = OnceLock::new();

fn metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        let registry = Registry::new();

        let cn_up = IntGaugeVec::new(Opts::new("cn_up", "Service health"), &["service"])
            .expect("cn_up metric");

        let http_requests_total = IntCounterVec::new(
            Opts::new("http_requests_total", "HTTP request count"),
            &["service", "route", "method", "status"],
        )
        .expect("http_requests_total metric");

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["service", "route", "method", "status"],
        )
        .expect("http_request_duration_seconds metric");

        let ws_connections = IntGaugeVec::new(
            Opts::new("ws_connections", "Active websocket connections"),
            &["service"],
        )
        .expect("ws_connections metric");

        let ws_unauthenticated_connections = IntGaugeVec::new(
            Opts::new(
                "ws_unauthenticated_connections",
                "Active websocket connections without successful AUTH",
            ),
            &["service"],
        )
        .expect("ws_unauthenticated_connections metric");

        let ws_req_total = IntCounterVec::new(
            Opts::new("ws_req_total", "Total websocket REQ messages"),
            &["service"],
        )
        .expect("ws_req_total metric");

        let ws_event_total = IntCounterVec::new(
            Opts::new("ws_event_total", "Total websocket EVENT messages"),
            &["service"],
        )
        .expect("ws_event_total metric");

        let ws_auth_disconnect_total = IntCounterVec::new(
            Opts::new(
                "ws_auth_disconnect_total",
                "Total websocket disconnects caused by auth transition enforcement",
            ),
            &["service", "reason"],
        )
        .expect("ws_auth_disconnect_total metric");

        let ingest_received_total = IntCounterVec::new(
            Opts::new("ingest_received_total", "Total ingest messages received"),
            &["service", "source"],
        )
        .expect("ingest_received_total metric");

        let ingest_rejected_total = IntCounterVec::new(
            Opts::new("ingest_rejected_total", "Total ingest messages rejected"),
            &["service", "reason"],
        )
        .expect("ingest_rejected_total metric");

        let gossip_received_total = IntCounterVec::new(
            Opts::new("gossip_received_total", "Total gossip messages received"),
            &["service"],
        )
        .expect("gossip_received_total metric");

        let gossip_sent_total = IntCounterVec::new(
            Opts::new("gossip_sent_total", "Total gossip messages sent"),
            &["service"],
        )
        .expect("gossip_sent_total metric");

        let bootstrap_hint_publish_total = IntCounterVec::new(
            Opts::new(
                "bootstrap_hint_publish_total",
                "Total bootstrap update hint publish outcomes",
            ),
            &["service", "channel", "result"],
        )
        .expect("bootstrap_hint_publish_total metric");

        let dedupe_hits_total =
            IntCounterVec::new(Opts::new("dedupe_hits_total", "Dedupe hits"), &["service"])
                .expect("dedupe_hits_total metric");

        let dedupe_misses_total = IntCounterVec::new(
            Opts::new("dedupe_misses_total", "Dedupe misses"),
            &["service"],
        )
        .expect("dedupe_misses_total metric");

        let auth_success_total = IntCounterVec::new(
            Opts::new("auth_success_total", "Authentication success count"),
            &["service"],
        )
        .expect("auth_success_total metric");

        let auth_failure_total = IntCounterVec::new(
            Opts::new("auth_failure_total", "Authentication failure count"),
            &["service"],
        )
        .expect("auth_failure_total metric");

        let consent_required_total = IntCounterVec::new(
            Opts::new(
                "consent_required_total",
                "Requests rejected due to missing consent",
            ),
            &["service"],
        )
        .expect("consent_required_total metric");

        let quota_exceeded_total = IntCounterVec::new(
            Opts::new(
                "quota_exceeded_total",
                "Requests rejected due to quota exceeded",
            ),
            &["service", "metric"],
        )
        .expect("quota_exceeded_total metric");

        let outbox_backlog = IntGaugeVec::new(
            Opts::new("outbox_backlog", "Outbox backlog by consumer"),
            &["service", "consumer"],
        )
        .expect("outbox_backlog metric");

        let outbox_consumer_batches_total = IntCounterVec::new(
            Opts::new(
                "outbox_consumer_batches_total",
                "Total outbox consumer batch outcomes",
            ),
            &["service", "consumer", "result"],
        )
        .expect("outbox_consumer_batches_total metric");

        let outbox_consumer_processing_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "outbox_consumer_processing_duration_seconds",
                "Outbox consumer processing duration in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["service", "consumer", "result"],
        )
        .expect("outbox_consumer_processing_duration_seconds metric");

        let outbox_consumer_batch_size = HistogramVec::new(
            HistogramOpts::new("outbox_consumer_batch_size", "Outbox consumer batch size").buckets(
                vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 200.0, 500.0, 1000.0],
            ),
            &["service", "consumer"],
        )
        .expect("outbox_consumer_batch_size metric");

        let suggest_stage_a_latency_ms = HistogramVec::new(
            HistogramOpts::new(
                "suggest_stage_a_latency_ms",
                "Community suggest Stage-A latency in milliseconds",
            )
            .buckets(vec![
                1.0, 2.5, 5.0, 10.0, 20.0, 50.0, 80.0, 120.0, 200.0, 400.0,
            ]),
            &["service", "backend"],
        )
        .expect("suggest_stage_a_latency_ms metric");

        let suggest_stage_b_latency_ms = HistogramVec::new(
            HistogramOpts::new(
                "suggest_stage_b_latency_ms",
                "Community suggest Stage-B latency in milliseconds",
            )
            .buckets(vec![
                1.0, 2.5, 5.0, 10.0, 20.0, 50.0, 80.0, 120.0, 200.0, 400.0,
            ]),
            &["service", "mode"],
        )
        .expect("suggest_stage_b_latency_ms metric");

        let suggest_block_filter_drop_count = IntCounterVec::new(
            Opts::new(
                "suggest_block_filter_drop_count",
                "Filtered suggest candidates dropped by block or mute filters",
            ),
            &["service", "backend", "reason"],
        )
        .expect("suggest_block_filter_drop_count metric");

        let backfill_processed_rows = IntGaugeVec::new(
            Opts::new(
                "backfill_processed_rows",
                "Rows processed by backfill jobs for each target",
            ),
            &["service", "target"],
        )
        .expect("backfill_processed_rows metric");

        let backfill_eta_seconds = IntGaugeVec::new(
            Opts::new(
                "backfill_eta_seconds",
                "Estimated seconds until current backfill target completes",
            ),
            &["service", "target"],
        )
        .expect("backfill_eta_seconds metric");

        let shadow_overlap_at_10 = HistogramVec::new(
            HistogramOpts::new(
                "shadow_overlap_at_10",
                "Search shadow-read overlap@10 between primary and secondary backends",
            )
            .buckets(vec![0.0, 0.1, 0.25, 0.4, 0.5, 0.7, 0.85, 1.0]),
            &["service", "endpoint", "primary_backend"],
        )
        .expect("shadow_overlap_at_10 metric");

        let shadow_latency_delta_ms = HistogramVec::new(
            HistogramOpts::new(
                "shadow_latency_delta_ms",
                "Absolute latency delta in milliseconds between primary and shadow backends",
            )
            .buckets(vec![
                0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 40.0, 80.0, 120.0, 200.0, 400.0,
            ]),
            &["service", "endpoint"],
        )
        .expect("shadow_latency_delta_ms metric");

        let search_dual_write_errors_total = IntCounterVec::new(
            Opts::new(
                "search_dual_write_errors_total",
                "Total dual-write side failures by backend and operation",
            ),
            &["service", "backend", "operation"],
        )
        .expect("search_dual_write_errors_total metric");

        let search_dual_write_retries_total = IntCounterVec::new(
            Opts::new(
                "search_dual_write_retries_total",
                "Total successful retries after dual-write side failures",
            ),
            &["service", "operation"],
        )
        .expect("search_dual_write_retries_total metric");

        registry
            .register(Box::new(cn_up.clone()))
            .expect("register cn_up");
        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("register http_requests_total");
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .expect("register http_request_duration_seconds");
        registry
            .register(Box::new(ws_connections.clone()))
            .expect("register ws_connections");
        registry
            .register(Box::new(ws_unauthenticated_connections.clone()))
            .expect("register ws_unauthenticated_connections");
        registry
            .register(Box::new(ws_req_total.clone()))
            .expect("register ws_req_total");
        registry
            .register(Box::new(ws_event_total.clone()))
            .expect("register ws_event_total");
        registry
            .register(Box::new(ws_auth_disconnect_total.clone()))
            .expect("register ws_auth_disconnect_total");
        registry
            .register(Box::new(ingest_received_total.clone()))
            .expect("register ingest_received_total");
        registry
            .register(Box::new(ingest_rejected_total.clone()))
            .expect("register ingest_rejected_total");
        registry
            .register(Box::new(gossip_received_total.clone()))
            .expect("register gossip_received_total");
        registry
            .register(Box::new(gossip_sent_total.clone()))
            .expect("register gossip_sent_total");
        registry
            .register(Box::new(bootstrap_hint_publish_total.clone()))
            .expect("register bootstrap_hint_publish_total");
        registry
            .register(Box::new(dedupe_hits_total.clone()))
            .expect("register dedupe_hits_total");
        registry
            .register(Box::new(dedupe_misses_total.clone()))
            .expect("register dedupe_misses_total");
        registry
            .register(Box::new(auth_success_total.clone()))
            .expect("register auth_success_total");
        registry
            .register(Box::new(auth_failure_total.clone()))
            .expect("register auth_failure_total");
        registry
            .register(Box::new(consent_required_total.clone()))
            .expect("register consent_required_total");
        registry
            .register(Box::new(quota_exceeded_total.clone()))
            .expect("register quota_exceeded_total");
        registry
            .register(Box::new(outbox_backlog.clone()))
            .expect("register outbox_backlog");
        registry
            .register(Box::new(outbox_consumer_batches_total.clone()))
            .expect("register outbox_consumer_batches_total");
        registry
            .register(Box::new(
                outbox_consumer_processing_duration_seconds.clone(),
            ))
            .expect("register outbox_consumer_processing_duration_seconds");
        registry
            .register(Box::new(outbox_consumer_batch_size.clone()))
            .expect("register outbox_consumer_batch_size");
        registry
            .register(Box::new(suggest_stage_a_latency_ms.clone()))
            .expect("register suggest_stage_a_latency_ms");
        registry
            .register(Box::new(suggest_stage_b_latency_ms.clone()))
            .expect("register suggest_stage_b_latency_ms");
        registry
            .register(Box::new(suggest_block_filter_drop_count.clone()))
            .expect("register suggest_block_filter_drop_count");
        registry
            .register(Box::new(backfill_processed_rows.clone()))
            .expect("register backfill_processed_rows");
        registry
            .register(Box::new(backfill_eta_seconds.clone()))
            .expect("register backfill_eta_seconds");
        registry
            .register(Box::new(shadow_overlap_at_10.clone()))
            .expect("register shadow_overlap_at_10");
        registry
            .register(Box::new(shadow_latency_delta_ms.clone()))
            .expect("register shadow_latency_delta_ms");
        registry
            .register(Box::new(search_dual_write_errors_total.clone()))
            .expect("register search_dual_write_errors_total");
        registry
            .register(Box::new(search_dual_write_retries_total.clone()))
            .expect("register search_dual_write_retries_total");

        Metrics {
            registry,
            cn_up,
            http_requests_total,
            http_request_duration_seconds,
            ws_connections,
            ws_unauthenticated_connections,
            ws_req_total,
            ws_event_total,
            ws_auth_disconnect_total,
            ingest_received_total,
            ingest_rejected_total,
            gossip_received_total,
            gossip_sent_total,
            bootstrap_hint_publish_total,
            dedupe_hits_total,
            dedupe_misses_total,
            auth_success_total,
            auth_failure_total,
            consent_required_total,
            quota_exceeded_total,
            outbox_backlog,
            outbox_consumer_batches_total,
            outbox_consumer_processing_duration_seconds,
            outbox_consumer_batch_size,
            suggest_stage_a_latency_ms,
            suggest_stage_b_latency_ms,
            suggest_block_filter_drop_count,
            backfill_processed_rows,
            backfill_eta_seconds,
            shadow_overlap_at_10,
            shadow_latency_delta_ms,
            search_dual_write_errors_total,
            search_dual_write_retries_total,
        }
    })
}

pub fn init(service_name: &'static str) {
    metrics().cn_up.with_label_values(&[service_name]).set(1);
}

pub fn record_http_request(
    service_name: &'static str,
    method: &str,
    route: &str,
    status: u16,
    duration: Duration,
) {
    let status_str = status.to_string();
    let labels = &[service_name, route, method, status_str.as_str()];
    let metrics = metrics();
    metrics.http_requests_total.with_label_values(labels).inc();
    metrics
        .http_request_duration_seconds
        .with_label_values(labels)
        .observe(duration.as_secs_f64());
}

pub fn inc_ws_connections(service_name: &'static str) {
    metrics()
        .ws_connections
        .with_label_values(&[service_name])
        .inc();
}

pub fn dec_ws_connections(service_name: &'static str) {
    metrics()
        .ws_connections
        .with_label_values(&[service_name])
        .dec();
}

pub fn inc_ws_unauthenticated_connections(service_name: &'static str) {
    metrics()
        .ws_unauthenticated_connections
        .with_label_values(&[service_name])
        .inc();
}

pub fn dec_ws_unauthenticated_connections(service_name: &'static str) {
    metrics()
        .ws_unauthenticated_connections
        .with_label_values(&[service_name])
        .dec();
}

pub fn inc_ws_req_total(service_name: &'static str) {
    metrics()
        .ws_req_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_ws_event_total(service_name: &'static str) {
    metrics()
        .ws_event_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_ws_auth_disconnect(service_name: &'static str, reason: &'static str) {
    metrics()
        .ws_auth_disconnect_total
        .with_label_values(&[service_name, reason])
        .inc();
}

pub fn inc_ingest_received(service_name: &'static str, source: &'static str) {
    metrics()
        .ingest_received_total
        .with_label_values(&[service_name, source])
        .inc();
}

pub fn inc_ingest_rejected(service_name: &'static str, reason: &'static str) {
    metrics()
        .ingest_rejected_total
        .with_label_values(&[service_name, reason])
        .inc();
}

pub fn inc_gossip_received(service_name: &'static str) {
    metrics()
        .gossip_received_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_gossip_sent(service_name: &'static str) {
    metrics()
        .gossip_sent_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_bootstrap_hint_publish(service_name: &'static str, channel: &str, result: &str) {
    metrics()
        .bootstrap_hint_publish_total
        .with_label_values(&[service_name, channel, result])
        .inc();
}

pub fn inc_dedupe_hit(service_name: &'static str) {
    metrics()
        .dedupe_hits_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_dedupe_miss(service_name: &'static str) {
    metrics()
        .dedupe_misses_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_auth_success(service_name: &'static str) {
    metrics()
        .auth_success_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_auth_failure(service_name: &'static str) {
    metrics()
        .auth_failure_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_consent_required(service_name: &'static str) {
    metrics()
        .consent_required_total
        .with_label_values(&[service_name])
        .inc();
}

pub fn inc_quota_exceeded(service_name: &'static str, metric: &str) {
    metrics()
        .quota_exceeded_total
        .with_label_values(&[service_name, metric])
        .inc();
}

pub fn set_outbox_backlog(service_name: &'static str, consumer: &str, backlog: i64) {
    metrics()
        .outbox_backlog
        .with_label_values(&[service_name, consumer])
        .set(backlog);
}

pub fn inc_outbox_consumer_batch_total(service_name: &'static str, consumer: &str, result: &str) {
    metrics()
        .outbox_consumer_batches_total
        .with_label_values(&[service_name, consumer, result])
        .inc();
}

pub fn observe_outbox_consumer_processing_duration(
    service_name: &'static str,
    consumer: &str,
    result: &str,
    duration: Duration,
) {
    metrics()
        .outbox_consumer_processing_duration_seconds
        .with_label_values(&[service_name, consumer, result])
        .observe(duration.as_secs_f64());
}

pub fn observe_outbox_consumer_batch_size(
    service_name: &'static str,
    consumer: &str,
    batch_size: usize,
) {
    metrics()
        .outbox_consumer_batch_size
        .with_label_values(&[service_name, consumer])
        .observe(batch_size as f64);
}

pub fn observe_suggest_stage_a_latency_ms(
    service_name: &'static str,
    backend: &str,
    duration: Duration,
) {
    metrics()
        .suggest_stage_a_latency_ms
        .with_label_values(&[service_name, backend])
        .observe(duration.as_secs_f64() * 1000.0);
}

pub fn observe_suggest_stage_b_latency_ms(
    service_name: &'static str,
    mode: &str,
    duration: Duration,
) {
    metrics()
        .suggest_stage_b_latency_ms
        .with_label_values(&[service_name, mode])
        .observe(duration.as_secs_f64() * 1000.0);
}

pub fn inc_suggest_block_filter_drop_count(
    service_name: &'static str,
    backend: &str,
    reason: &str,
    count: u64,
) {
    if count == 0 {
        return;
    }
    metrics()
        .suggest_block_filter_drop_count
        .with_label_values(&[service_name, backend, reason])
        .inc_by(count);
}

pub fn set_backfill_processed_rows(service_name: &'static str, target: &str, processed_rows: i64) {
    metrics()
        .backfill_processed_rows
        .with_label_values(&[service_name, target])
        .set(processed_rows.max(0));
}

pub fn set_backfill_eta_seconds(service_name: &'static str, target: &str, eta_seconds: f64) {
    let eta = if eta_seconds.is_finite() {
        eta_seconds.max(0.0)
    } else {
        0.0
    };
    metrics()
        .backfill_eta_seconds
        .with_label_values(&[service_name, target])
        .set(eta.round() as i64);
}

pub fn observe_shadow_overlap_at_10(
    service_name: &'static str,
    endpoint: &str,
    primary_backend: &str,
    overlap: f64,
) {
    if !overlap.is_finite() {
        return;
    }
    metrics()
        .shadow_overlap_at_10
        .with_label_values(&[service_name, endpoint, primary_backend])
        .observe(overlap.clamp(0.0, 1.0));
}

pub fn observe_shadow_latency_delta_ms(service_name: &'static str, endpoint: &str, delta_ms: f64) {
    if !delta_ms.is_finite() {
        return;
    }
    metrics()
        .shadow_latency_delta_ms
        .with_label_values(&[service_name, endpoint])
        .observe(delta_ms.abs());
}

pub fn inc_search_dual_write_error(service_name: &'static str, backend: &str, operation: &str) {
    metrics()
        .search_dual_write_errors_total
        .with_label_values(&[service_name, backend, operation])
        .inc();
}

pub fn inc_search_dual_write_retry(service_name: &'static str, operation: &str) {
    metrics()
        .search_dual_write_retries_total
        .with_label_values(&[service_name, operation])
        .inc();
}

pub fn metrics_response(service_name: &'static str) -> impl IntoResponse {
    init(service_name);
    let metrics = metrics();
    let metric_families = metrics.registry.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    if encoder.encode(&metric_families, &mut buffer).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            HeaderMap::new(),
            "failed to encode metrics".to_string(),
        );
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "text/plain; version=0.0.4".parse().unwrap(),
    );
    (
        StatusCode::OK,
        headers,
        String::from_utf8_lossy(&buffer).to_string(),
    )
}

#[derive(Clone)]
pub struct MetricsLayer {
    service_name: &'static str,
}

impl MetricsLayer {
    pub fn new(service_name: &'static str) -> Self {
        Self { service_name }
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
    service_name: &'static str,
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsService {
            inner,
            service_name: self.service_name,
        }
    }
}

impl<S, ReqBody, ResBody> Service<axum::http::Request<ReqBody>> for MetricsService<S>
where
    S: Service<axum::http::Request<ReqBody>, Response = axum::response::Response<ResBody>>
        + Send
        + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = axum::response::Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: axum::http::Request<ReqBody>) -> Self::Future {
        let service_name = self.service_name;
        let method = request.method().to_string();
        let route = request.uri().path().to_string();
        let start = Instant::now();
        let fut = self.inner.call(request);
        Box::pin(async move {
            match fut.await {
                Ok(response) => {
                    record_http_request(
                        service_name,
                        &method,
                        &route,
                        response.status().as_u16(),
                        start.elapsed(),
                    );
                    Ok(response)
                }
                Err(err) => {
                    record_http_request(service_name, &method, &route, 500, start.elapsed());
                    Err(err)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn metrics_response_sets_content_type() {
        let response = metrics_response("cn-test").into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "text/plain; version=0.0.4");
    }
}
