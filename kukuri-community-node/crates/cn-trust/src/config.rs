use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TrustRuntimeConfig {
    pub enabled: bool,
    pub consumer_batch_size: i64,
    pub consumer_poll_seconds: u64,
    pub report_window_days: i64,
    pub report_weight: f64,
    pub label_weight: f64,
    pub report_score_normalization: f64,
    pub communication_window_days: i64,
    pub communication_score_normalization: f64,
    pub interaction_weights: HashMap<i32, f64>,
    pub attestation_exp_seconds: i64,
    pub schedule_poll_seconds: u64,
    pub report_schedule_interval_seconds: i64,
    pub communication_schedule_interval_seconds: i64,
}

impl TrustRuntimeConfig {
    pub fn from_json(value: &Value) -> Self {
        let consumer = value.get("consumer").and_then(|v| v.as_object());
        let report = value.get("report_based").and_then(|v| v.as_object());
        let communication = value
            .get("communication_density")
            .and_then(|v| v.as_object());
        let attestation = value.get("attestation").and_then(|v| v.as_object());
        let jobs = value.get("jobs").and_then(|v| v.as_object());

        let interaction_weights = communication
            .and_then(|map| map.get("interaction_weights"))
            .and_then(|v| v.as_object())
            .map(|weights| {
                weights
                    .iter()
                    .filter_map(|(kind, value)| {
                        let kind = kind.parse::<i32>().ok()?;
                        let weight = value.as_f64().unwrap_or(0.0);
                        if weight > 0.0 {
                            Some((kind, weight))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<i32, f64>>()
            })
            .unwrap_or_else(|| {
                let mut defaults = HashMap::new();
                defaults.insert(1, 1.0);
                defaults.insert(6, 0.5);
                defaults.insert(7, 0.3);
                defaults
            });

        Self {
            enabled: value.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
            consumer_batch_size: consumer
                .and_then(|map| map.get("batch_size"))
                .and_then(|v| v.as_i64())
                .unwrap_or(200),
            consumer_poll_seconds: consumer
                .and_then(|map| map.get("poll_interval_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(5),
            report_window_days: report
                .and_then(|map| map.get("window_days"))
                .and_then(|v| v.as_i64())
                .unwrap_or(30)
                .max(1),
            report_weight: report
                .and_then(|map| map.get("report_weight"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .max(0.0),
            label_weight: report
                .and_then(|map| map.get("label_weight"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .max(0.0),
            report_score_normalization: report
                .and_then(|map| map.get("score_normalization"))
                .and_then(|v| v.as_f64())
                .unwrap_or(10.0)
                .max(1.0),
            communication_window_days: communication
                .and_then(|map| map.get("window_days"))
                .and_then(|v| v.as_i64())
                .unwrap_or(30)
                .max(1),
            communication_score_normalization: communication
                .and_then(|map| map.get("score_normalization"))
                .and_then(|v| v.as_f64())
                .unwrap_or(20.0)
                .max(1.0),
            interaction_weights,
            attestation_exp_seconds: attestation
                .and_then(|map| map.get("exp_seconds"))
                .and_then(|v| v.as_i64())
                .unwrap_or(86400)
                .max(60),
            schedule_poll_seconds: jobs
                .and_then(|map| map.get("schedule_poll_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(30),
            report_schedule_interval_seconds: jobs
                .and_then(|map| map.get("report_based_interval_seconds"))
                .and_then(|v| v.as_i64())
                .unwrap_or(86400)
                .max(60),
            communication_schedule_interval_seconds: jobs
                .and_then(|map| map.get("communication_interval_seconds"))
                .and_then(|v| v.as_i64())
                .unwrap_or(86400)
                .max(60),
        }
    }
}
