use super::trending_metrics_job::TrendingMetricsRunStats;
use crate::shared::error::AppError;
use prometheus::{
    Encoder, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry, TextEncoder,
};
use std::sync::Arc;
use std::time::Duration;

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn prometheus_err(err: prometheus::Error) -> AppError {
    AppError::Internal(err.to_string())
}

pub struct TrendingMetricsRecorder {
    registry: Arc<Registry>,
    encoder: TextEncoder,
    runs_total: IntCounter,
    failures_total: IntCounter,
    topics_upserted: IntGauge,
    expired_records: IntGauge,
    last_success_ms: IntGauge,
    last_failure_ms: IntGauge,
    window_start_ms: IntGauge,
    window_end_ms: IntGauge,
    lag_ms: IntGauge,
    weight_posts: Gauge,
    weight_unique_authors: Gauge,
    weight_boosts: Gauge,
    duration_seconds: Option<Histogram>,
}

impl TrendingMetricsRecorder {
    pub fn new(emit_histogram: bool) -> Result<Self, AppError> {
        let registry = Registry::new_custom(Some("kukuri".into()), None).map_err(prometheus_err)?;

        let runs_total = IntCounter::with_opts(Opts::new(
            "trending_metrics_job_runs_total",
            "Total number of successful trending metrics job executions",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(runs_total.clone()))
            .map_err(prometheus_err)?;

        let failures_total = IntCounter::with_opts(Opts::new(
            "trending_metrics_job_failures_total",
            "Total number of failed trending metrics job executions",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(failures_total.clone()))
            .map_err(prometheus_err)?;

        let topics_upserted = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_topics_upserted",
            "Latest topics_upserted count emitted by the job",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(topics_upserted.clone()))
            .map_err(prometheus_err)?;

        let expired_records = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_expired_records",
            "Latest expired records count removed by the job",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(expired_records.clone()))
            .map_err(prometheus_err)?;

        let last_success_ms = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_last_success_timestamp",
            "Unix timestamp in milliseconds of the last successful execution",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(last_success_ms.clone()))
            .map_err(prometheus_err)?;

        let last_failure_ms = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_last_failure_timestamp",
            "Unix timestamp in milliseconds of the last failed execution",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(last_failure_ms.clone()))
            .map_err(prometheus_err)?;

        let window_start_ms = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_window_start_ms",
            "Window start timestamp (milliseconds) for the last successful execution",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(window_start_ms.clone()))
            .map_err(prometheus_err)?;

        let window_end_ms = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_window_end_ms",
            "Window end timestamp (milliseconds) for the last successful execution",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(window_end_ms.clone()))
            .map_err(prometheus_err)?;

        let lag_ms = IntGauge::with_opts(Opts::new(
            "trending_metrics_job_lag_ms",
            "Lag in milliseconds between now and the emitted window end",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(lag_ms.clone()))
            .map_err(prometheus_err)?;

        let weight_posts = Gauge::with_opts(Opts::new(
            "trending_metrics_score_weight_posts",
            "Configured score weight applied to posts",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(weight_posts.clone()))
            .map_err(prometheus_err)?;

        let weight_unique_authors = Gauge::with_opts(Opts::new(
            "trending_metrics_score_weight_unique_authors",
            "Configured score weight applied to unique authors",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(weight_unique_authors.clone()))
            .map_err(prometheus_err)?;

        let weight_boosts = Gauge::with_opts(Opts::new(
            "trending_metrics_score_weight_boosts",
            "Configured score weight applied to boosts",
        ))
        .map_err(prometheus_err)?;
        registry
            .register(Box::new(weight_boosts.clone()))
            .map_err(prometheus_err)?;

        let duration_seconds = if emit_histogram {
            let histogram = Histogram::with_opts(
                HistogramOpts::new(
                    "trending_metrics_job_duration_seconds",
                    "Observed duration of trending metrics job executions",
                )
                .buckets(vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            )
            .map_err(prometheus_err)?;
            registry
                .register(Box::new(histogram.clone()))
                .map_err(prometheus_err)?;
            Some(histogram)
        } else {
            None
        };

        Ok(Self {
            registry: Arc::new(registry),
            encoder: TextEncoder::new(),
            runs_total,
            failures_total,
            topics_upserted,
            expired_records,
            last_success_ms,
            last_failure_ms,
            window_start_ms,
            window_end_ms,
            lag_ms,
            weight_posts,
            weight_unique_authors,
            weight_boosts,
            duration_seconds,
        })
    }

    pub fn record_success(&self, duration: Duration, stats: &TrendingMetricsRunStats) {
        self.runs_total.inc();
        self.topics_upserted.set(stats.topics_upserted as i64);
        self.expired_records.set(stats.expired_records as i64);
        self.last_success_ms.set(now_millis());
        self.window_start_ms.set(stats.window_start_millis);
        self.window_end_ms.set(stats.window_end_millis);
        self.lag_ms.set(stats.lag_millis);
        self.weight_posts.set(stats.score_weights.posts);
        self.weight_unique_authors
            .set(stats.score_weights.unique_authors);
        self.weight_boosts.set(stats.score_weights.boosts);
        if let Some(histogram) = &self.duration_seconds {
            histogram.observe(duration.as_secs_f64());
        }
    }

    pub fn record_failure(&self, duration: Duration) {
        self.failures_total.inc();
        self.last_failure_ms.set(now_millis());
        if let Some(histogram) = &self.duration_seconds {
            histogram.observe(duration.as_secs_f64());
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, AppError> {
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        self.encoder
            .encode(&metric_families, &mut buffer)
            .map_err(prometheus_err)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ScoreWeights;
    use std::str;

    fn contains_metric(haystack: &str, key: &str, value: &str) -> bool {
        haystack
            .lines()
            .any(|line| line.trim().starts_with(key) && line.trim().ends_with(value))
    }

    #[test]
    fn record_success_and_failure_update_metrics() {
        let recorder = TrendingMetricsRecorder::new(true).expect("recorder");
        let stats = TrendingMetricsRunStats {
            topics_upserted: 3,
            expired_records: 1,
            cutoff_millis: 0,
            window_start_millis: 0,
            window_end_millis: 0,
            lag_millis: 0,
            score_weights: ScoreWeights::default(),
        };

        recorder.record_success(Duration::from_millis(1200), &stats);
        let snapshot = String::from_utf8(recorder.encode().expect("encode")).expect("utf8");
        assert!(
            contains_metric(&snapshot, "kukuri_trending_metrics_job_runs_total", "1"),
            "runs_total metric missing: {snapshot}"
        );
        assert!(snapshot.contains("trending_metrics_job_topics_upserted 3"));

        recorder.record_failure(Duration::from_millis(800));
        let snapshot = String::from_utf8(recorder.encode().expect("encode")).expect("utf8");
        assert!(
            contains_metric(&snapshot, "kukuri_trending_metrics_job_failures_total", "1"),
            "failures_total metric missing: {snapshot}"
        );
    }
}
