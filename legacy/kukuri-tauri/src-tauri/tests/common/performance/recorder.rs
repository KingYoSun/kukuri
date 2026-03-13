use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{Context, Result};
use chrono::{Timelike, Utc};
use serde::Serialize;

#[derive(Serialize)]
struct PerformanceReport<'a> {
    scenario: &'a str,
    timestamp: String,
    iterations: u64,
    metrics: &'a BTreeMap<String, f64>,
    notes: &'a BTreeMap<String, String>,
}

#[derive(Default)]
pub struct PerformanceRecorder {
    scenario: String,
    iterations: u64,
    metrics: BTreeMap<String, f64>,
    notes: BTreeMap<String, String>,
}

impl PerformanceRecorder {
    pub fn new<S: Into<String>>(scenario: S) -> Self {
        Self {
            scenario: scenario.into(),
            iterations: 0,
            metrics: BTreeMap::new(),
            notes: BTreeMap::new(),
        }
    }

    pub fn iterations(mut self, value: u64) -> Self {
        self.iterations = value;
        self
    }

    pub fn metric(mut self, key: &str, value: f64) -> Self {
        self.metrics.insert(key.to_string(), value);
        self
    }

    pub fn note(mut self, key: &str, value: impl Into<String>) -> Self {
        self.notes.insert(key.to_string(), value.into());
        self
    }

    pub fn write(self) -> Result<PathBuf> {
        let output_dir = resolve_output_dir();
        fs::create_dir_all(&output_dir).context("create performance output directory")?;

        let timestamp = Utc::now();
        let filename = format!(
            "{}-{}.json",
            timestamp.format("%Y%m%d%H%M%S"),
            sanitize_filename(&self.scenario)
        );
        let path = output_dir.join(filename);

        let report = PerformanceReport {
            scenario: &self.scenario,
            timestamp: timestamp
                .with_nanosecond(0)
                .unwrap_or(timestamp)
                .to_rfc3339(),
            iterations: self.iterations,
            metrics: &self.metrics,
            notes: &self.notes,
        };

        let file = File::create(&path).context("create performance report file")?;
        serde_json::to_writer_pretty(file, &report).context("write performance report")?;
        Ok(path)
    }
}

fn resolve_output_dir() -> PathBuf {
    env::var("KUKURI_PERFORMANCE_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("test-results/performance"))
}

fn sanitize_filename(source: &str) -> String {
    let mut sanitized: String = source
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            'A'..='Z' => c.to_ascii_lowercase(),
            '_' | '-' => c,
            _ if c.is_whitespace() => '_',
            _ => '-',
        })
        .collect();

    if sanitized.is_empty() {
        sanitized.push_str("scenario");
    }

    while sanitized.starts_with(['-', '_']) {
        sanitized.remove(0);
    }

    if sanitized.is_empty() {
        sanitized.push_str("scenario");
    }

    sanitized
}

pub fn duration_secs(duration: std::time::Duration) -> f64 {
    let secs = duration.as_secs_f64();
    if secs <= f64::EPSILON {
        f64::EPSILON
    } else {
        secs
    }
}
