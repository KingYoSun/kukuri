use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use kukuri_cn_core::{
    AdmissionMode, COMMUNITY_NODE_AUTH_SERVICE_NAME, IndexScopeKind, IndexingRequestStatus,
    connect_postgres,
};

mod commands;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long, env = "COMMUNITY_NODE_DATABASE_URL")]
    database_url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Prepare,
    Migrate,
    SeedPolicies,
    SetAuthRollout {
        #[arg(long, default_value = COMMUNITY_NODE_AUTH_SERVICE_NAME)]
        service: String,
        #[arg(long)]
        mode: AuthModeArg,
        #[arg(long)]
        enforce_at: Option<String>,
        #[arg(long, default_value_t = 900)]
        grace_seconds: i64,
        #[arg(long, default_value_t = 10)]
        ws_auth_timeout_seconds: i64,
    },
    /// 受信した通報（#370）を確認する。
    Reports {
        #[command(subcommand)]
        action: ReportsAction,
    },
    /// 入会制御（招待 / whitelist / ban）を運用する（#383）。
    Admission {
        #[command(subcommand)]
        action: AdmissionAction,
    },
    /// index が引き受ける supported topic / 許可 channel を運用する（#413）。
    SupportedTopic {
        #[command(subcommand)]
        action: SupportedTopicAction,
    },
    /// user からの indexing request を確認・承認する（#413）。
    IndexingRequest {
        #[command(subcommand)]
        action: IndexingRequestAction,
    },
    /// relation graph（trust / relation foundation, #415）を運用する。
    Relation {
        #[command(subcommand)]
        action: RelationAction,
    },
    /// moderation advisory（risk signal）の appeal / operator レビューを運用する（#420）。
    Moderation {
        #[command(subcommand)]
        action: ModerationCliAction,
    },
    /// 公開ノードの準備完了判定を実行時情報込みで検査する（#616）。
    Readiness {
        /// operator-config.yaml のパス。
        #[arg(long, default_value = "operator-config.yaml")]
        config: PathBuf,
        #[arg(long, default_value = "public-node")]
        profile: String,
        /// 疎通確認結果の再利用期限（秒）。期限内は外部プロバイダを叩かない。
        #[arg(long, default_value_t = 900)]
        probe_ttl_secs: u64,
        /// 保存済みの疎通確認結果を無視して必ず再実行する。
        #[arg(long)]
        force_probe: bool,
    },
}

/// moderation advisory の運用操作（ADR 0028 §2.3 / §2.8）。
///
/// `edit` / `reissue` は operator レビュー機能で、env
/// `COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW=true` の明示的有効化が必要。
#[derive(Debug, Subcommand)]
enum ModerationCliAction {
    /// risk signal を新着順に一覧する（visibility を問わない node-local 参照）。
    ListSignals {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
    /// risk signal の詳細を表示する。
    Show {
        #[arg(long)]
        id: String,
    },
    /// 申し立てを受理して Disputed にする（None→Disputed。冪等）。
    Dispute {
        #[arg(long)]
        id: String,
    },
    /// 申し立てを認容して Cleared にする（Disputed→Cleared。trust 寄与が戻る）。
    Clear {
        #[arg(long)]
        id: String,
    },
    /// 申し立てを棄却して None に戻す（Disputed→None。確定寄与を維持）。
    Reject {
        #[arg(long)]
        id: String,
    },
    /// 検知メタデータを直接編集する（operator レビュー。node-local advisory の是正）。
    Edit {
        #[arg(long)]
        id: String,
        #[arg(long)]
        category: Option<SafetyCategoryArg>,
        #[arg(long)]
        severity: Option<SeverityArg>,
        #[arg(long)]
        confidence: Option<u8>,
        /// 失効時刻（RFC3339）。
        #[arg(long)]
        expires_at: Option<String>,
    },
    /// 訂正 signal を再発行する（旧 signal は失効。operator レビュー）。
    Reissue {
        #[arg(long)]
        id: String,
        #[arg(long)]
        category: Option<SafetyCategoryArg>,
        #[arg(long)]
        severity: Option<SeverityArg>,
        #[arg(long)]
        confidence: Option<u8>,
        #[arg(long)]
        visibility: Option<VisibilityArg>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SafetyCategoryArg {
    Csam,
    Cse,
    Grooming,
    Nsfw,
    Spam,
    Malware,
    Phishing,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SeverityArg {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum VisibilityArg {
    Local,
    SubscribedNodes,
    Public,
}

#[derive(Debug, Subcommand)]
enum RelationAction {
    Analyze {
        #[arg(long, default_value_t = kukuri_cn_indexer::DEFAULT_ANALYSIS_LIMIT)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
enum SupportedTopicAction {
    Add {
        #[arg(long)]
        kind: IndexScopeKindArg,
        #[arg(long)]
        id: String,
    },
    Remove {
        #[arg(long)]
        kind: IndexScopeKindArg,
        #[arg(long)]
        id: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum IndexingRequestAction {
    List {
        #[arg(long)]
        status: Option<IndexingRequestStatusArg>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
    Approve {
        #[arg(long)]
        id: String,
    },
    Reject {
        #[arg(long)]
        id: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum IndexScopeKindArg {
    PublicTopic,
    PrivateChannel,
}

impl From<IndexScopeKindArg> for IndexScopeKind {
    fn from(value: IndexScopeKindArg) -> Self {
        match value {
            IndexScopeKindArg::PublicTopic => Self::PublicTopic,
            IndexScopeKindArg::PrivateChannel => Self::PrivateChannel,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum IndexingRequestStatusArg {
    Pending,
    Approved,
    Rejected,
}

impl From<IndexingRequestStatusArg> for IndexingRequestStatus {
    fn from(value: IndexingRequestStatusArg) -> Self {
        match value {
            IndexingRequestStatusArg::Pending => Self::Pending,
            IndexingRequestStatusArg::Approved => Self::Approved,
            IndexingRequestStatusArg::Rejected => Self::Rejected,
        }
    }
}

#[derive(Debug, Subcommand)]
enum AdmissionAction {
    Show,
    SetMode {
        #[arg(long)]
        mode: AdmissionModeArg,
    },
    Invite {
        #[command(subcommand)]
        action: InviteAction,
    },
    Allow {
        #[command(subcommand)]
        action: AllowAction,
    },
    Ban {
        #[command(subcommand)]
        action: BanAction,
    },
}

#[derive(Debug, Subcommand)]
enum InviteAction {
    Issue {
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        max_uses: Option<i32>,
        #[arg(long)]
        expires_at: Option<String>,
    },
    List,
    Revoke {
        #[arg(long)]
        code: String,
    },
}

#[derive(Debug, Subcommand)]
enum AllowAction {
    Add {
        #[arg(long)]
        pubkey: String,
        #[arg(long)]
        label: Option<String>,
    },
    Remove {
        #[arg(long)]
        pubkey: String,
    },
    List,
}

#[derive(Debug, Subcommand)]
enum BanAction {
    Add {
        #[arg(long)]
        pubkey: String,
    },
    Remove {
        #[arg(long)]
        pubkey: String,
    },
    List,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AdmissionModeArg {
    Open,
    Invite,
    Whitelist,
}

impl From<AdmissionModeArg> for AdmissionMode {
    fn from(value: AdmissionModeArg) -> Self {
        match value {
            AdmissionModeArg::Open => Self::Open,
            AdmissionModeArg::Invite => Self::Invite,
            AdmissionModeArg::Whitelist => Self::Whitelist,
        }
    }
}

#[derive(Debug, Subcommand)]
enum ReportsAction {
    List {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },
    Show {
        #[arg(long)]
        id: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum AuthModeArg {
    Off,
    Required,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let pool = connect_postgres(cli.database_url.as_str()).await?;
    commands::dispatch(&pool, cli.command).await
}
