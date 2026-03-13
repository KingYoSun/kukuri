use anyhow::{Context, Result, bail};
use chrono::Utc;
use kukuri_lib::{
    AppConfig, ConnectionPool, SqliteRepository, TopicMetricsRecord, TopicMetricsRepository,
    TopicMetricsSnapshot, ops::p2p::metrics,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use tokio::runtime::Runtime;

const DEFAULT_LIMIT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    P2P,
    Trending,
}

#[derive(Debug, Clone)]
struct CliOptions {
    output: Option<PathBuf>,
    pretty: bool,
    job: JobKind,
    limit: usize,
    database_url: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
struct ScoreWeightsSummary {
    posts: f64,
    unique_authors: f64,
    boosts: f64,
}

#[derive(Debug, serde::Serialize)]
struct TrendingJobReport {
    job: &'static str,
    generated_at_ms: i64,
    collected_at_ms: i64,
    limit: usize,
    metrics_count: usize,
    score_weights: ScoreWeightsSummary,
    window_start_ms: Option<i64>,
    window_end_ms: Option<i64>,
    window_duration_ms: Option<i64>,
    lag_ms: Option<i64>,
    topics: Vec<TopicMetricsRecord>,
}

fn usage() -> &'static str {
    "Usage: p2p_metrics_export [--job <p2p|trending>] [--output <path>] [--pretty] [--limit <n>] [--database-url <url>]"
}

fn write_output(path: &Path, data: &str) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(path, data).with_context(|| format!("Failed to write {}", path.display()))
}

fn emit_payload(target: Option<&Path>, payload: &str) -> Result<()> {
    if let Some(path) = target {
        write_output(path, payload)?;
        println!("Metrics written to {}", path.display());
    } else {
        println!("{payload}");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let options = parse_args(args.into_iter())?;

    match options.job {
        JobKind::P2P => export_p2p(&options),
        JobKind::Trending => export_trending(&options),
    }
}

fn export_p2p(options: &CliOptions) -> Result<()> {
    let snapshot = metrics::snapshot_full();
    let payload = to_json(&snapshot, options.pretty)?;
    emit_payload(options.output.as_deref(), &payload)
}

fn export_trending(options: &CliOptions) -> Result<()> {
    let database_url = resolve_database_url(options);
    let weights = current_score_weights();
    let rt = Runtime::new().context("Failed to create Tokio runtime")?;
    let report = rt.block_on(async {
        collect_trending_report(&database_url, options.limit, weights)
            .await
            .with_context(|| format!("Failed to collect trending metrics from {database_url}"))
    })?;

    let payload = to_json(&report, options.pretty)?;
    emit_payload(options.output.as_deref(), &payload)
}

fn to_json<T: serde::Serialize>(value: &T, pretty: bool) -> Result<String> {
    if pretty {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        Ok(serde_json::to_string(value)?)
    }
}

fn parse_args<I>(args: I) -> Result<CliOptions>
where
    I: IntoIterator<Item = String>,
{
    let mut output: Option<PathBuf> = None;
    let mut pretty = false;
    let mut job = JobKind::P2P;
    let mut limit = DEFAULT_LIMIT;
    let mut database_url: Option<String> = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                let path = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--output requires a path\n{}", usage()))?;
                output = Some(PathBuf::from(path));
            }
            "--pretty" => {
                pretty = true;
            }
            "--job" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--job requires a value\n{}", usage()))?;
                job = parse_job(&value)?;
            }
            "--limit" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--limit requires a value\n{}", usage()))?;
                limit = parse_limit(&value)?;
            }
            "--database-url" => {
                let value = iter.next().ok_or_else(|| {
                    anyhow::anyhow!("--database-url requires a value\n{}", usage())
                })?;
                database_url = Some(value);
            }
            "-h" | "--help" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => {
                bail!("Unknown argument: {other}\n{}", usage());
            }
        }
    }

    Ok(CliOptions {
        output,
        pretty,
        job,
        limit,
        database_url,
    })
}

fn parse_job(value: &str) -> Result<JobKind> {
    match value.to_ascii_lowercase().as_str() {
        "p2p" => Ok(JobKind::P2P),
        "trending" => Ok(JobKind::Trending),
        other => bail!("Unknown job: {other}. Expected 'p2p' or 'trending'."),
    }
}

fn parse_limit(value: &str) -> Result<usize> {
    let parsed: usize = value
        .parse()
        .with_context(|| format!("Invalid limit '{value}'. Expected a positive integer."))?;
    if parsed == 0 {
        bail!("--limit must be greater than 0");
    }
    Ok(parsed)
}

fn resolve_database_url(options: &CliOptions) -> String {
    if let Some(url) = &options.database_url {
        return url.clone();
    }
    if let Ok(env_url) = env::var("DATABASE_URL")
        && !env_url.trim().is_empty()
    {
        return env_url;
    }
    AppConfig::from_env().database.url
}

fn current_score_weights() -> ScoreWeightsSummary {
    let cfg = AppConfig::from_env();
    ScoreWeightsSummary {
        posts: cfg.metrics.score_weights.posts,
        unique_authors: cfg.metrics.score_weights.unique_authors,
        boosts: cfg.metrics.score_weights.boosts,
    }
}

async fn collect_trending_report(
    database_url: &str,
    limit: usize,
    weights: ScoreWeightsSummary,
) -> Result<TrendingJobReport> {
    let pool = ConnectionPool::new(database_url)
        .await
        .with_context(|| format!("Failed to connect to database at {database_url}"))?;
    let repository = SqliteRepository::new(pool);
    let snapshot = repository
        .list_recent_metrics(limit)
        .await
        .context("Failed to query topic metrics snapshot")?;

    Ok(build_report(snapshot, limit, weights))
}

fn build_report(
    snapshot: Option<TopicMetricsSnapshot>,
    limit: usize,
    weights: ScoreWeightsSummary,
) -> TrendingJobReport {
    let now_ms = current_unix_ms();

    if let Some(snapshot) = snapshot {
        let duration = snapshot
            .window_end
            .saturating_sub(snapshot.window_start)
            .max(0);
        let lag = now_ms.saturating_sub(snapshot.window_end).max(0);
        let metrics = snapshot.metrics;
        TrendingJobReport {
            job: "trending_metrics",
            generated_at_ms: snapshot.window_end,
            collected_at_ms: now_ms,
            limit,
            metrics_count: metrics.len(),
            score_weights: weights,
            window_start_ms: Some(snapshot.window_start),
            window_end_ms: Some(snapshot.window_end),
            window_duration_ms: Some(duration),
            lag_ms: Some(lag),
            topics: metrics,
        }
    } else {
        TrendingJobReport {
            job: "trending_metrics",
            generated_at_ms: now_ms,
            collected_at_ms: now_ms,
            limit,
            metrics_count: 0,
            score_weights: weights,
            window_start_ms: None,
            window_end_ms: None,
            window_duration_ms: None,
            lag_ms: None,
            topics: Vec::new(),
        }
    }
}

fn current_unix_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults() {
        let opts = parse_args(Vec::<String>::new()).expect("options");
        assert_eq!(opts.job, JobKind::P2P);
        assert_eq!(opts.limit, DEFAULT_LIMIT);
        assert!(opts.output.is_none());
    }

    #[test]
    fn parses_trending_job_with_options() {
        let opts = parse_args(
            vec![
                "--job".into(),
                "trending".into(),
                "--limit".into(),
                "10".into(),
                "--database-url".into(),
                "sqlite::memory:".into(),
                "--output".into(),
                "out.json".into(),
                "--pretty".into(),
            ]
            .into_iter(),
        )
        .expect("options");

        assert_eq!(opts.job, JobKind::Trending);
        assert_eq!(opts.limit, 10);
        assert_eq!(opts.database_url.as_deref(), Some("sqlite::memory:"));
        assert!(opts.pretty);
        assert_eq!(opts.output.as_deref(), Some(Path::new("out.json")));
    }

    #[test]
    fn rejects_invalid_job() {
        let err = parse_args(vec!["--job".into(), "unknown".into()].into_iter()).unwrap_err();
        assert!(format!("{err}").contains("Unknown job"));
    }

    #[test]
    fn reject_zero_limit() {
        let err = parse_args(vec!["--limit".into(), "0".into()].into_iter()).unwrap_err();
        assert!(format!("{err}").contains("greater than 0"));
    }

    #[test]
    fn trending_report_uses_snapshot_window_end() {
        let snapshot = TopicMetricsSnapshot {
            window_start: 1_000,
            window_end: 2_000,
            metrics: Vec::new(),
        };
        let weights = ScoreWeightsSummary {
            posts: 0.6,
            unique_authors: 0.3,
            boosts: 0.1,
        };

        let report = build_report(Some(snapshot), 25, weights);

        assert_eq!(report.generated_at_ms, 2_000);
        assert_eq!(report.window_start_ms, Some(1_000));
        assert_eq!(report.window_end_ms, Some(2_000));
        assert!(report.collected_at_ms >= report.generated_at_ms);
        assert_eq!(report.score_weights.posts, 0.6);
    }
}
