use kukuri_cn_core::PgModerationBudget;
use kukuri_cn_safety::{ScanError, provider::VideoFrameExtractor};
use kukuri_cn_safety_openai::{
    ModerationClient, ModerationConfig, ModerationCredentials, ModerationInput,
};
use kukuri_cn_safety_video::{
    DecoderFailure, FfmpegVideoExtractor, VideoExtractConfig, classify_failure,
};
use sqlx::PgPool;
use std::sync::Arc;

pub(super) struct PreparedProbe {
    client: ModerationClient,
    extractor: FfmpegVideoExtractor,
    pub fingerprint: String,
}

impl PreparedProbe {
    /// The error is an operator-facing detail built only from fixed text.
    pub fn new(pool: &PgPool, slot: &str) -> Result<Self, String> {
        if slot != "general" {
            return Err("openai-moderation は general slot 専用です".into());
        }
        let config = ModerationConfig::from_env()
            .map_err(|_| "設定不備: COMMUNITY_NODE_MODERATION_* の値を確認".to_string())?;
        let credentials = ModerationCredentials::from_env()
            .map_err(|_| "資格情報未設定: COMMUNITY_NODE_VLM_API_KEY を確認".to_string())?;
        let extractor = VideoExtractConfig::from_env()
            .and_then(FfmpegVideoExtractor::new)
            .map_err(|error| format!("動画decoderの初期化に失敗: {}", decoder_reason(&error)))?;
        let client = ModerationClient::new(
            config,
            credentials,
            Arc::new(PgModerationBudget::new(pool.clone())),
        )
        .map_err(|_| "OpenAI clientの初期化に失敗".to_string())?;
        Ok(Self::from_parts(client, extractor))
    }

    fn from_parts(client: ModerationClient, extractor: FfmpegVideoExtractor) -> Self {
        let fingerprint = format!(
            "{}|{}",
            client.config().fingerprint(),
            extractor.config_fingerprint()
        );
        Self {
            client,
            extractor,
            fingerprint,
        }
    }

    pub async fn run(&self) -> super::readiness::ProbeOutcome {
        let result = self.check().await;
        super::readiness::ProbeOutcome {
            pass: result.is_ok(),
            detail: result
                .err()
                .unwrap_or_else(|| "MP4/WebM decode、認証、本文/画像応答の解析に成功".into()),
        }
    }

    /// Stops at the first failed stage, so a decoder failure sends nothing to OpenAI.
    async fn check(&self) -> Result<(), String> {
        let frame = self
            .extractor
            .readiness_probe()
            .await
            .map_err(|error| format!("動画decoderの確認に失敗: {}", decoder_reason(&error)))?;
        let deadline = tokio::time::Instant::now() + self.client.config().scan_timeout;
        self.client
            .moderate(
                ModerationInput::Text("kukuri readiness: benign connectivity check"),
                deadline,
            )
            .await
            .map_err(|error| format!("OpenAI本文の確認に失敗: {}", openai_reason(&error)))?;
        self.client
            .moderate(ModerationInput::Image(&frame.bytes), deadline)
            .await
            .map_err(|error| format!("OpenAI画像の確認に失敗: {}", openai_reason(&error)))?;
        Ok(())
    }
}

fn decoder_reason(error: &ScanError) -> &'static str {
    match classify_failure(error) {
        DecoderFailure::InvalidConfig => "抽出設定の値が不正",
        DecoderFailure::DecoderMissing => "ffmpeg/ffprobe の実行ファイルが無いか不正",
        DecoderFailure::ScratchBusy => "作業領域の保守処理との競合が待機上限を超過",
        DecoderFailure::ScratchUnavailable => "作業領域（専用tmpfs）が無いか利用できない",
        DecoderFailure::QueueTimeout => "抽出の順番待ちが上限を超過",
        DecoderFailure::WarmUpTimeout => "decoder初回起動の準備が時間切れ",
        DecoderFailure::ProbeTimeout => "ffprobe が時間切れ",
        DecoderFailure::DecodeTimeout => "ffmpeg が時間切れ",
        DecoderFailure::SpawnFailed => "隔離環境でdecoderを起動できない",
        DecoderFailure::DecoderExited => "decoderが異常終了または資源上限を超過",
        DecoderFailure::Rejected => "同梱動画または抽出結果の検証に失敗",
        DecoderFailure::Cancelled => "処理が中断された",
        DecoderFailure::Internal => "内部エラー",
    }
}

/// Maps only fixed messages and a parsed status code; never echoes the error text.
fn openai_reason(error: &ScanError) -> String {
    let (ScanError::Unavailable(message)
    | ScanError::Timeout(message)
    | ScanError::Protocol(message)) = error;
    if let Some(status) = http_status(message) {
        return match status {
            401 | 403 => format!("認証拒否 (HTTP {status})"),
            429 => "頻度制限 (HTTP 429)".into(),
            500..=599 => format!("プロバイダ側エラー (HTTP {status})"),
            _ => format!("予期しない応答 (HTTP {status})"),
        };
    }
    match (error, message.as_str()) {
        (_, "moderation budget is exhausted") => "共有予算の枯渇".into(),
        (_, "persistent moderation budget is unavailable") => "共有予算のDBを利用できない".into(),
        (_, m) if m.contains("budget") => "共有予算の設定または予約量が不正".into(),
        (ScanError::Timeout(_), _) => "時間切れ".into(),
        (_, "moderation transport failed" | "moderation retries exhausted") => "通信失敗".into(),
        (_, "moderation queue is full" | "moderation client is stopping") => {
            "処理の順番待ちが上限".into()
        }
        (ScanError::Protocol(_), m)
            if m.contains("response") || m.contains("JSON") || m.contains("category") =>
        {
            "応答の解釈に失敗".into()
        }
        (ScanError::Protocol(_), _) => "要求または応答の検証に失敗".into(),
        _ => "利用不可（分類外）".into(),
    }
}

fn http_status(message: &str) -> Option<u16> {
    let rest = message.strip_prefix("moderation HTTP ")?;
    let digits = rest.get(..3)?;
    let status = digits.parse::<u16>().ok()?;
    (digits.bytes().all(|b| b.is_ascii_digit())
        && (100..=599).contains(&status)
        && rest[3..].chars().next().is_none_or(|c| c == ' '))
    .then_some(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_failures_are_classified_without_echoing_messages() {
        let cases = [
            (
                ScanError::Protocol("moderation HTTP 401".into()),
                "認証拒否 (HTTP 401)",
            ),
            (
                ScanError::Protocol("moderation HTTP 403".into()),
                "認証拒否 (HTTP 403)",
            ),
            (
                ScanError::Protocol("moderation HTTP 404".into()),
                "予期しない応答 (HTTP 404)",
            ),
            (
                ScanError::Unavailable("moderation HTTP 429 after bounded retries".into()),
                "頻度制限 (HTTP 429)",
            ),
            (
                ScanError::Unavailable("moderation HTTP 503 after bounded retries".into()),
                "プロバイダ側エラー (HTTP 503)",
            ),
            (
                ScanError::Unavailable("moderation budget is exhausted".into()),
                "共有予算の枯渇",
            ),
            (
                ScanError::Unavailable("persistent moderation budget is unavailable".into()),
                "共有予算のDBを利用できない",
            ),
            (
                ScanError::Protocol("moderation input exceeds token reservation budget".into()),
                "共有予算の設定または予約量が不正",
            ),
            (
                ScanError::Timeout("moderation scan deadline exceeded".into()),
                "時間切れ",
            ),
            (
                ScanError::Unavailable("moderation transport failed".into()),
                "通信失敗",
            ),
            (
                ScanError::Unavailable("moderation queue is full".into()),
                "処理の順番待ちが上限",
            ),
            (
                ScanError::Protocol("invalid moderation JSON".into()),
                "応答の解釈に失敗",
            ),
            (
                ScanError::Protocol("moderation response is too large".into()),
                "応答の解釈に失敗",
            ),
            (
                ScanError::Protocol("moderation request is too large".into()),
                "要求または応答の検証に失敗",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(openai_reason(&error), expected, "{error}");
        }
        // Unknown or malformed text never reaches the operator-facing detail.
        for error in [
            ScanError::Unavailable("sk-live-secret echoed by a future change".into()),
            ScanError::Protocol("moderation HTTP 4011 sk-live-secret".into()),
            ScanError::Protocol("moderation HTTP 40x sk-live-secret".into()),
            ScanError::Timeout("sk-live-secret".into()),
        ] {
            let detail = openai_reason(&error);
            assert!(!detail.contains("sk-live-secret"), "{detail}");
            assert!(!detail.contains("HTTP 40"), "{detail}");
        }
    }

    #[test]
    fn decoder_failures_have_distinct_operator_details() {
        let details: std::collections::BTreeSet<_> = [
            ScanError::Protocol("invalid video extraction bounds".into()),
            ScanError::Unavailable("video decoder executable missing".into()),
            ScanError::Unavailable("video tmpfs maintenance is busy".into()),
            ScanError::Unavailable("video scratch directory is not a private tmpfs".into()),
            ScanError::Timeout("video extraction queue wait exceeded".into()),
            ScanError::Timeout("video decoder warm-up deadline exceeded".into()),
            ScanError::Timeout("video probe deadline exceeded".into()),
            ScanError::Timeout("video decoder deadline exceeded".into()),
            ScanError::Unavailable("cannot start sandboxed decoder".into()),
            ScanError::Protocol("decoder failed or exceeded resource limits".into()),
            ScanError::Protocol("unsupported video codec".into()),
            ScanError::Unavailable("video scan cancelled".into()),
            ScanError::Unavailable("sk-live-secret".into()),
        ]
        .iter()
        .map(decoder_reason)
        .collect();
        assert_eq!(details.len(), 13);
        assert!(details.iter().all(|detail| !detail.contains("sk-live")));
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;
        use async_trait::async_trait;
        use kukuri_cn_safety_openai::{BudgetConfig, ModerationBudget};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        #[derive(Default)]
        struct CountingBudget(AtomicUsize);

        #[async_trait]
        impl ModerationBudget for CountingBudget {
            async fn reserve(&self, _: u32, _: &BudgetConfig) -> Result<Duration, ScanError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Duration::ZERO)
            }
        }

        fn private_tmpfs() -> tempfile::TempDir {
            use std::os::unix::fs::PermissionsExt;
            let root = tempfile::tempdir_in("/dev/shm").expect("tmpfs");
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))
                .expect("private scratch");
            root
        }

        fn probe(
            server: &MockServer,
            budget: Arc<CountingBudget>,
            video: VideoExtractConfig,
        ) -> PreparedProbe {
            let config = ModerationConfig {
                api_base_url: server.uri(),
                http_timeout: Duration::from_secs(5),
                scan_timeout: Duration::from_secs(10),
                ..ModerationConfig::default()
            };
            let client = ModerationClient::new(
                config,
                ModerationCredentials::new("sk-live-secret").expect("credential"),
                budget,
            )
            .expect("client");
            PreparedProbe::from_parts(client, FfmpegVideoExtractor::new(video).expect("decoder"))
        }

        #[tokio::test]
        async fn decoder_failure_is_reported_before_any_openai_use() {
            use std::os::unix::fs::PermissionsExt;
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200))
                .expect(0)
                .mount(&server)
                .await;
            let root = private_tmpfs();
            let programs = tempfile::tempdir().expect("wrapper directory");
            let ffprobe = programs.path().join("ffprobe-broken");
            std::fs::write(
                &ffprobe,
                "#!/bin/sh\ncase \"$*\" in *-version*) exit 0 ;; esac\nexit 1\n",
            )
            .expect("wrapper");
            std::fs::set_permissions(&ffprobe, std::fs::Permissions::from_mode(0o700))
                .expect("executable wrapper");
            let budget = Arc::new(CountingBudget::default());
            let outcome = probe(
                &server,
                budget.clone(),
                VideoExtractConfig {
                    ffprobe,
                    temporary_root: root.path().to_owned(),
                    ..Default::default()
                },
            )
            .run()
            .await;
            assert!(!outcome.pass);
            assert_eq!(
                outcome.detail,
                "動画decoderの確認に失敗: decoderが異常終了または資源上限を超過"
            );
            assert_eq!(budget.0.load(Ordering::SeqCst), 0);
        }

        #[tokio::test]
        async fn openai_rejection_names_the_stage_and_status_only() {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/moderations"))
                .respond_with(ResponseTemplate::new(401).set_body_string("sk-live-secret echoed"))
                .expect(1)
                .mount(&server)
                .await;
            let root = private_tmpfs();
            let budget = Arc::new(CountingBudget::default());
            let outcome = probe(
                &server,
                budget.clone(),
                VideoExtractConfig {
                    temporary_root: root.path().to_owned(),
                    ..Default::default()
                },
            )
            .run()
            .await;
            assert!(!outcome.pass);
            assert_eq!(
                outcome.detail,
                "OpenAI本文の確認に失敗: 認証拒否 (HTTP 401)"
            );
            assert_eq!(budget.0.load(Ordering::SeqCst), 1);
        }
    }
}
