use kukuri_cn_core::PgModerationBudget;
use kukuri_cn_safety::{ScanError, provider::VideoFrameExtractor};
use kukuri_cn_safety_openai::{
    ModerationClient, ModerationConfig, ModerationCredentials, ModerationInput,
};
use kukuri_cn_safety_video::{FfmpegVideoExtractor, VideoExtractConfig};
use sqlx::PgPool;
use std::sync::Arc;

pub(super) struct PreparedProbe {
    client: ModerationClient,
    extractor: FfmpegVideoExtractor,
    pub fingerprint: String,
}

impl PreparedProbe {
    pub fn new(pool: &PgPool, slot: &str) -> Result<Self, ScanError> {
        if slot != "general" {
            return Err(ScanError::Protocol(
                "openai-moderation is only valid in the general slot".into(),
            ));
        }
        let config = ModerationConfig::from_env()?;
        let credentials = ModerationCredentials::from_env()?;
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig::from_env()?)?;
        let fingerprint = format!(
            "{}|{}",
            config.fingerprint(),
            extractor.config_fingerprint()
        );
        let client = ModerationClient::new(
            config,
            credentials,
            Arc::new(PgModerationBudget::new(pool.clone())),
        )?;
        Ok(Self {
            client,
            extractor,
            fingerprint,
        })
    }

    pub async fn run(&self) -> super::readiness::ProbeOutcome {
        let result = async {
            let frame = self.extractor.readiness_probe().await?;
            let deadline = tokio::time::Instant::now() + self.client.config().scan_timeout;
            self.client
                .moderate(
                    ModerationInput::Text("kukuri readiness: benign connectivity check"),
                    deadline,
                )
                .await?;
            self.client
                .moderate(ModerationInput::Image(&frame.bytes), deadline)
                .await?;
            Ok::<_, ScanError>(())
        }
        .await;
        super::readiness::ProbeOutcome {
            pass: result.is_ok(),
            detail: match result {
                Ok(()) => "MP4/WebM decode、認証、本文/画像応答の解析に成功".into(),
                Err(_) => "OpenAI/動画decoderの確認に失敗（設定・依存・共有予算を確認）".into(),
            },
        }
    }
}
