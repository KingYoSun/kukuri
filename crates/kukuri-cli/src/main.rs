use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};
use kukuri_desktop_runtime::{
    AGE_ATTESTATION_VERSION, APP_LEGAL_DOCUMENTS, AgeAttestationRecord, AppConsentDocumentRecord,
    AppConsentStore, ProfileError, ProfileLease, app_consent_satisfied, current_unix_seconds,
    load_app_consent_store, resolve_cli_profile, save_app_consent_store,
};

#[cfg(target_os = "linux")]
mod daemon;

#[derive(Parser)]
#[command(name = "kukuri-cli", version)]
struct Cli {
    /// CLI専用profile。KUKURI_INSTANCEと同じselectorとして扱う。
    #[arg(long)]
    profile: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Daemon(DaemonArgs),
    Consent(ConsentArgs),
}

#[derive(Args)]
struct DaemonArgs {
    #[command(subcommand)]
    command: DaemonCommand,
}

#[derive(Subcommand)]
enum DaemonCommand {
    /// foregroundで常駐プロセスを実行する。
    Run,
    /// systemd user instanceを開始する。
    Start,
    /// systemd user instanceを停止する。
    Stop,
    /// systemd user instanceの状態を表示する。
    Status,
}

#[derive(Args)]
struct ConsentArgs {
    #[command(subcommand)]
    command: ConsentCommand,
}

#[derive(Subcommand)]
enum ConsentCommand {
    Status,
    Accept(ConsentAcceptArgs),
}

#[derive(Args)]
struct ConsentAcceptArgs {
    /// 現行利用規約とプライバシーポリシーへの同意を明示する。
    #[arg(long)]
    accept_documents: bool,
    /// 18歳以上であることの自己申告を明示する。
    #[arg(long)]
    age_confirmed: bool,
    #[arg(long, default_value = "ja")]
    language: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}: {}", error.code, error.message);
        std::process::exit(error.exit_code);
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let custom_app_data_selected = std::env::var("KUKURI_APP_DATA_DIR")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    let profile = resolve_profile(cli.profile.as_deref()).map_err(CliError::from_profile)?;
    match cli.command {
        Command::Consent(args) => run_consent(profile, args.command),
        Command::Daemon(args) => run_daemon(profile, args.command, custom_app_data_selected),
    }
}

fn resolve_profile(
    argument_profile: Option<&str>,
) -> Result<kukuri_desktop_runtime::ClientProfile, ProfileError> {
    let environment_instance = std::env::var("KUKURI_INSTANCE").ok();
    let environment_app_data_dir = std::env::var("KUKURI_APP_DATA_DIR").ok();
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    resolve_cli_profile(
        argument_profile,
        environment_instance.as_deref(),
        environment_app_data_dir.as_deref(),
        xdg_data_home.as_deref(),
        home.as_deref(),
    )
}

fn run_consent(
    profile: kukuri_desktop_runtime::ClientProfile,
    command: ConsentCommand,
) -> Result<(), CliError> {
    let lease = ProfileLease::acquire(profile).map_err(CliError::from_profile)?;
    let consent_db_path = lease.profile().app_data_dir.join("kukuri.db");
    match command {
        ConsentCommand::Status => {
            let store = load_app_consent_store(&consent_db_path);
            println!(
                "{}",
                if app_consent_satisfied(&store) {
                    "satisfied"
                } else {
                    "consent_required"
                }
            );
            Ok(())
        }
        ConsentCommand::Accept(args) => accept_consents(&consent_db_path, args),
    }
}

fn accept_consents(path: &Path, args: ConsentAcceptArgs) -> Result<(), CliError> {
    if !args.accept_documents || !args.age_confirmed {
        return Err(CliError::new(
            "consent_confirmation_required",
            "--accept-documents and --age-confirmed are both required",
            2,
        ));
    }
    let language = args.language.trim();
    if language.is_empty() {
        return Err(CliError::new(
            "invalid_consent_language",
            "--language must not be empty",
            2,
        ));
    }
    let accepted_at = current_unix_seconds();
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let store = AppConsentStore {
        records: APP_LEGAL_DOCUMENTS
            .iter()
            .map(|(slug, version)| AppConsentDocumentRecord {
                slug: (*slug).to_string(),
                version: *version,
                accepted_at,
                language: language.to_string(),
                app_version: app_version.clone(),
            })
            .collect(),
        age_attestations: vec![AgeAttestationRecord {
            version: AGE_ATTESTATION_VERSION,
            attested_at: accepted_at,
            language: language.to_string(),
            app_version,
        }],
    };
    save_app_consent_store(path, &store)
        .map_err(|error| CliError::new("consent_persist_failed", error, 1))?;
    println!("accepted");
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_daemon(
    profile: kukuri_desktop_runtime::ClientProfile,
    command: DaemonCommand,
    custom_app_data_selected: bool,
) -> Result<(), CliError> {
    if custom_app_data_selected && !matches!(&command, DaemonCommand::Run) {
        return Err(CliError::new(
            "custom_profile_systemd_unsupported",
            "KUKURI_APP_DATA_DIR can be used only with `daemon run`; systemd actions require a named profile",
            2,
        ));
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::new("runtime_start_failed", error.to_string(), 1))?;
    runtime.block_on(daemon::run(profile, command))
}

#[cfg(not(target_os = "linux"))]
fn run_daemon(
    _profile: kukuri_desktop_runtime::ClientProfile,
    _command: DaemonCommand,
    _custom_app_data_selected: bool,
) -> Result<(), CliError> {
    Err(CliError::new(
        "unsupported_platform",
        "kukuri daemon is supported only on Linux",
        2,
    ))
}

#[derive(Debug)]
struct CliError {
    code: &'static str,
    message: String,
    exit_code: i32,
}

impl CliError {
    fn new(code: &'static str, message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            code,
            message: message.into(),
            exit_code,
        }
    }

    fn from_profile(error: ProfileError) -> Self {
        Self::new(error.code(), error.to_string(), 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_requires_both_explicit_confirmations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = accept_consents(
            &dir.path().join("kukuri.db"),
            ConsentAcceptArgs {
                accept_documents: true,
                age_confirmed: false,
                language: "ja".to_string(),
            },
        )
        .expect_err("age confirmation");
        assert_eq!(error.code, "consent_confirmation_required");
    }

    #[test]
    fn consent_acceptance_writes_current_documents_and_age() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("kukuri.db");
        accept_consents(
            &path,
            ConsentAcceptArgs {
                accept_documents: true,
                age_confirmed: true,
                language: "ja".to_string(),
            },
        )
        .expect("accept consent");
        assert!(app_consent_satisfied(&load_app_consent_store(&path)));
    }
}
