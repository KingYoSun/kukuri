use crate::service::*;

impl AppService {
    /// #858: 成人向け表現の表示設定(既定 OFF)。desktop-runtime が永続値を起動時に
    /// 反映し、設定変更時にも呼ぶ。
    pub fn set_adult_content_display_enabled(&self, enabled: bool) {
        self.adult_content_display_enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn adult_content_display_enabled(&self) -> bool {
        self.adult_content_display_enabled
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub async fn blob_media_payload(
        &self,
        hash: &str,
        mime: &str,
    ) -> Result<Option<BlobMediaPayload>> {
        let hash = hash.trim();
        if hash.is_empty() {
            warn!(mime = %mime, "blob media payload fetch skipped because hash was blank");
            return Ok(None);
        }
        info!(hash = %hash, mime = %mime, "blob media payload fetch requested");
        let blob_hash = kukuri_core::BlobHash::new(hash.to_string());
        // #858 fail-closed バックストップ: 成人向けラベル付き投稿の添付として観測済みの
        // hash は、表示設定が OFF の間はネットワーク取得もローカル読み出しも行わない。
        // ON の場合も ephemeral fetch でローカル blob store へ永続化しない(ADR 0046)。
        let adult_labeled = self
            .services
            .projection_store
            .is_adult_media_hash(&blob_hash)
            .await?;
        if adult_labeled && !self.adult_content_display_enabled() {
            info!(
                hash = %hash,
                mime = %mime,
                "blob media payload fetch blocked: adult-labeled media while display is disabled"
            );
            return Ok(None);
        }
        let fetch_result = if adult_labeled {
            self.services
                .blob_service
                .fetch_blob_ephemeral(&blob_hash)
                .await
        } else {
            self.services.blob_service.fetch_blob(&blob_hash).await
        };
        let bytes = match fetch_result {
            Ok(Some(bytes)) => {
                info!(
                    hash = %hash,
                    mime = %mime,
                    byte_len = bytes.len(),
                    "blob media payload fetch hit"
                );
                bytes
            }
            Ok(None) => {
                warn!(hash = %hash, mime = %mime, "blob media payload fetch miss");
                return Ok(None);
            }
            Err(error) => {
                warn!(
                    hash = %hash,
                    mime = %mime,
                    error = %error,
                    "blob media payload fetch failed"
                );
                return Err(error);
            }
        };
        Ok(Some(BlobMediaPayload {
            bytes_base64: BASE64_STANDARD.encode(bytes),
            mime: mime.to_string(),
        }))
    }

    pub async fn blob_preview_data_url(&self, hash: &str, mime: &str) -> Result<Option<String>> {
        let Some(payload) = self.blob_media_payload(hash, mime).await? else {
            return Ok(None);
        };
        Ok(Some(format!(
            "data:{};base64,{}",
            payload.mime, payload.bytes_base64
        )))
    }
}
