use crate::service::*;

impl AppService {
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
        let bytes = match self
            .blob_service
            .fetch_blob(&kukuri_core::BlobHash::new(hash.to_string()))
            .await
        {
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
