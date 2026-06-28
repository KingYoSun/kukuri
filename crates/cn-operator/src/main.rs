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
use clap::{Parser, Subcommand};

use kukuri_cn_operator::{
    Availability, SAMPLE_CONFIG, check_drift, generate_all, generate_tfvars, load_and_validate,
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
    }
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
