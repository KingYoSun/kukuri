//! `cn-operator` CLI。operator config から運営者向け文書を生成・検証する。
//!
//! サブコマンド:
//! - `init`            : サンプル operator-config.yaml を出力する
//! - `validate-config` : config を検証する（Phase B 承認ガードを含む）
//! - `generate-docs`   : 文書群を出力ディレクトリへ生成する
//! - `generate-tfvars` : deploy セクションから terraform.tfvars を生成する（#380）
//! - `check-disclosures`: config から再生成した結果と既存生成物の drift を検出する

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use kukuri_cn_operator::{
    Availability, SAMPLE_CONFIG, check_drift, evaluate_public_node_readiness, generate_all,
    generate_tfvars, load_and_validate,
};

#[derive(Debug, Parser)]
#[command(
    name = "cn-operator",
    about = "kukuri community node operator docs generator"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// サンプル operator-config.yaml を出力する。
    Init {
        /// 出力先（既定: 標準出力）。
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// operator config を検証する。
    ValidateConfig {
        #[arg(long, default_value = "operator-config.yaml")]
        config: PathBuf,
    },
    /// 運営者向け文書群を生成する。
    GenerateDocs {
        #[arg(long, default_value = "operator-config.yaml")]
        config: PathBuf,
        #[arg(long, default_value = "dist/operator-docs")]
        out_dir: PathBuf,
    },
    /// deploy セクションから terraform.tfvars を生成する（#380）。
    GenerateTfvars {
        #[arg(long, default_value = "operator-config.yaml")]
        config: PathBuf,
        /// 出力先（既定: 標準出力）。
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// config から再生成した結果と既存生成物の drift を検出する。
    CheckDisclosures {
        #[arg(long, default_value = "operator-config.yaml")]
        config: PathBuf,
        #[arg(long, default_value = "dist/operator-docs")]
        out_dir: PathBuf,
    },
    /// safety readiness / provider 経路を検査する。
    Safety {
        #[command(subcommand)]
        action: SafetyCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SafetyCommand {
    /// public-node safety readiness を operator-config から静的検査する。
    Readiness {
        #[arg(long, default_value = "operator-config.yaml")]
        config: PathBuf,
        #[arg(long, default_value = "public-node")]
        profile: String,
    },
    /// mock provider で scan -> route -> verdict 経路を検査する。
    TestProvider {
        #[arg(long, default_value = "blob-test")]
        subject_id: String,
        #[arg(long, value_enum, default_value_t = TestProviderScenario::KnownMatch)]
        scenario: TestProviderScenario,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum TestProviderScenario {
    KnownMatch,
    NoKnownMatch,
    Unavailable,
    SuspectedCsam,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { out } => {
            match out {
                Some(path) => {
                    fs::write(&path, SAMPLE_CONFIG).with_context(|| {
                        format!("{} への書き込みに失敗しました", path.display())
                    })?;
                    println!("サンプル config を {} に出力しました", path.display());
                }
                None => print!("{SAMPLE_CONFIG}"),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::ValidateConfig { config } => {
            let resolved = load_config(&config)?;
            println!("config は有効です: {}", config.display());
            print_planned_warning(&resolved);
            Ok(ExitCode::SUCCESS)
        }
        Command::GenerateDocs { config, out_dir } => {
            let resolved = load_config(&config)?;
            fs::create_dir_all(&out_dir)
                .with_context(|| format!("{} の作成に失敗しました", out_dir.display()))?;
            let files = generate_all(&resolved);
            for file in &files {
                let path = out_dir.join(&file.filename);
                fs::write(&path, &file.content)
                    .with_context(|| format!("{} の書き込みに失敗しました", path.display()))?;
            }
            println!(
                "{} 個の文書を {} に生成しました",
                files.len(),
                out_dir.display()
            );
            print_planned_warning(&resolved);
            Ok(ExitCode::SUCCESS)
        }
        Command::GenerateTfvars { config, out } => {
            let resolved = load_config(&config)?;
            let tfvars = generate_tfvars(&resolved)?;
            match out {
                Some(path) => {
                    fs::write(&path, &tfvars).with_context(|| {
                        format!("{} への書き込みに失敗しました", path.display())
                    })?;
                    println!("terraform.tfvars を {} に出力しました", path.display());
                }
                None => print!("{tfvars}"),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::CheckDisclosures { config, out_dir } => {
            let resolved = load_config(&config)?;
            let report = check_drift(&resolved, &out_dir)?;
            println!("{}", report.summary());
            if report.is_clean() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::FAILURE)
            }
        }
        Command::Safety { action } => run_safety(action),
    }
}

fn run_safety(action: SafetyCommand) -> Result<ExitCode> {
    match action {
        SafetyCommand::Readiness { config, profile } => {
            let resolved = load_config(&config)?;
            let report = evaluate_public_node_readiness(&resolved, profile.as_str());
            println!(
                "safety readiness profile={} ready={} static_ok={} fail={} unknown={}",
                report.profile,
                report.is_ready(),
                report.static_checks_pass(),
                report.fail_count(),
                report.unknown_count()
            );
            for check in &report.checks {
                println!("{}  {}  {}", check.status.key(), check.id, check.detail);
            }
            if report.has_blocking_failures() {
                // static config に不備（fail）がある。設定で解消すべき。
                Ok(ExitCode::FAILURE)
            } else if report.is_ready() {
                println!("OK: すべての readiness check を満たしています。");
                Ok(ExitCode::SUCCESS)
            } else {
                // fail は無いが unknown が残る。runtime / provider 接続後に確定する項目。
                println!(
                    "NOTE: static config の check は満たしています。unknown の {} 項目は provider / runtime 接続後に確定します（public indexing 解禁前に再検査が必要）。",
                    report.unknown_count()
                );
                Ok(ExitCode::SUCCESS)
            }
        }
        SafetyCommand::TestProvider {
            subject_id,
            scenario,
        } => run_safety_test_provider(subject_id, scenario),
    }
}

#[cfg(feature = "safety-mock")]
fn run_safety_test_provider(
    subject_id: String,
    scenario: TestProviderScenario,
) -> Result<ExitCode> {
    use kukuri_cn_safety::{
        MockSafetyProvider, ProviderScanRequest, SafetyCategory, SafetyPolicy, SafetyProvider,
        SafetyProviderCapability, SubjectKind, route,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("tokio runtime の作成に失敗しました")?;
    let (provider, request) = match scenario {
        TestProviderScenario::KnownMatch => (
            MockSafetyProvider::known_csam("mock-known-csam")
                .with_known_hash_match(subject_id.as_str()),
            ProviderScanRequest::for_subject(SubjectKind::Blob, subject_id.as_str()),
        ),
        TestProviderScenario::NoKnownMatch => (
            MockSafetyProvider::known_csam("mock-known-csam")
                .with_no_known_match(subject_id.as_str()),
            ProviderScanRequest::for_subject(SubjectKind::Blob, subject_id.as_str()),
        ),
        TestProviderScenario::Unavailable => (
            MockSafetyProvider::known_csam("mock-known-csam").default_unavailable(),
            ProviderScanRequest::for_subject(SubjectKind::Blob, subject_id.as_str()),
        ),
        TestProviderScenario::SuspectedCsam => (
            MockSafetyProvider::with_capabilities(
                "mock-unknown-csam",
                vec![SafetyProviderCapability::NovelCsamImageClassifier],
            )
            .with_score(
                subject_id.as_str(),
                SafetyProviderCapability::NovelCsamImageClassifier,
                SafetyCategory::Csam,
                90,
            ),
            ProviderScanRequest::for_subject(SubjectKind::Blob, subject_id.as_str()),
        ),
    };
    let scan = runtime.block_on(async { provider.scan(&request).await })?;
    let policy = SafetyPolicy::public_node_default();
    let verdict = route(std::slice::from_ref(&scan), &policy, "mock-scanned-at");
    println!("provider: {}", scan.provider);
    println!("capability: {:?}", scan.capability);
    println!("outcome: {:?}", scan.outcome);
    println!("verdict_action: {:?}", verdict.action);
    println!("reason_code: {:?}", verdict.reason_code);
    println!("critical: {}", verdict.critical);
    println!("indexable: {}", verdict.is_indexable());
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(feature = "safety-mock"))]
fn run_safety_test_provider(
    _subject_id: String,
    _scenario: TestProviderScenario,
) -> Result<ExitCode> {
    eprintln!(
        "safety test-provider は `cargo run -p kukuri-cn-operator --features safety-mock -- safety test-provider` で実行してください"
    );
    Ok(ExitCode::FAILURE)
}

fn load_config(path: &PathBuf) -> Result<kukuri_cn_operator::ResolvedConfig> {
    let yaml = fs::read_to_string(path)
        .with_context(|| format!("{} の読み込みに失敗しました", path.display()))?;
    load_and_validate(&yaml)
}

fn print_planned_warning(resolved: &kukuri_cn_operator::ResolvedConfig) {
    let planned = resolved.enabled_planned_capabilities();
    if !planned.is_empty() {
        let names = planned
            .iter()
            .map(|c| c.key())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "注意: 計画中（未実装）capability が有効です: {names}（{}として扱われます）",
            Availability::Planned.label_ja()
        );
    }
}
