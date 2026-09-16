use std::{io::Cursor, sync::Arc, time::Duration};

use async_trait::async_trait;
use image::{AnimationDecoder, ImageDecoder, ImageFormat, ImageReader, codecs::jpeg::JpegEncoder};
use kukuri_cn_safety::{
    MediaFetcher, ProviderScanRequest, ProviderScanResult, SafetyProvider,
    SafetyProviderCapability, ScanError, ScanInputKind, ScanOutcome, provider::VideoFrameExtractor,
};
use tokio::{
    sync::Semaphore,
    time::{Instant, timeout_at},
};

use crate::{ModerationClient, ModerationInput, PROVIDER_NAME, config::invalid};

pub struct OpenAiModerationProvider {
    client: Arc<ModerationClient>,
    fetcher: Option<Arc<dyn MediaFetcher>>,
    video: Option<Arc<dyn VideoFrameExtractor>>,
    media_jobs: Semaphore,
    queue: Semaphore,
}

impl std::fmt::Debug for OpenAiModerationProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiModerationProvider")
            .field("has_video", &self.video.is_some())
            .finish_non_exhaustive()
    }
}

impl OpenAiModerationProvider {
    pub fn new(client: Arc<ModerationClient>) -> Self {
        Self {
            queue: Semaphore::new(client.config().queue_capacity),
            client,
            fetcher: None,
            video: None,
            media_jobs: Semaphore::new(1),
        }
    }

    pub fn with_media_fetcher(mut self, fetcher: Arc<dyn MediaFetcher>) -> Self {
        self.fetcher = Some(fetcher);
        self
    }

    pub fn with_video_extractor(mut self, video: Arc<dyn VideoFrameExtractor>) -> Self {
        self.video = Some(video);
        self
    }

    async fn scan_inner(
        &self,
        request: &ProviderScanRequest,
        deadline: Instant,
        guard: Option<&dyn kukuri_cn_safety::provider::ScanReferenceGuard>,
    ) -> Result<ProviderScanResult, ScanError> {
        if let Some(hint) = request.media_hint.as_deref() {
            if request.text.as_ref().is_some_and(|text| !text.is_empty()) {
                return Err(invalid("moderation cannot mix media and post text"));
            }
            let _job = timeout_at(
                deadline.min(Instant::now() + Duration::from_secs(60)),
                self.media_jobs.acquire(),
            )
            .await
            .map_err(|_| ScanError::Timeout("media scan queue wait exceeded".into()))?
            .map_err(|_| ScanError::Unavailable("media scan is stopping".into()))?;
            let fetcher = self
                .fetcher
                .as_ref()
                .ok_or_else(|| ScanError::Unavailable("moderation media fetcher missing".into()))?;
            if let Some(guard) = guard {
                guard.check().await?;
            }
            let media = fetcher.fetch(hint, request.media_mime.as_deref()).await?;
            if let Some(guard) = guard {
                guard.check().await?;
            }
            if media.bytes.len() > 32 * 1024 * 1024 {
                return Err(invalid("moderation media is too large"));
            }
            if is_video(&media.bytes) {
                let video = self
                    .video
                    .as_ref()
                    .ok_or_else(|| ScanError::Unavailable("video extractor missing".into()))?;
                let extracted = video.extract(&media.bytes).await?;
                if extracted.frames.is_empty() || extracted.frames.len() > 8 {
                    return Err(invalid("invalid extracted frame count"));
                }
                let frames = extracted.frames.len() as u16;
                let mut combined = None;
                for frame in &extracted.frames {
                    if frame.content_type != "image/jpeg" {
                        return Err(invalid("video frame is not JPEG"));
                    }
                    let result = self
                        .client
                        .moderate_guarded(ModerationInput::Image(&frame.bytes), deadline, guard)
                        .await?;
                    match &mut combined {
                        Some(assessment) => crate::ModerationAssessment::merge(assessment, result)?,
                        None => combined = Some(result),
                    }
                }
                let mut result = combined.expect("non-empty extracted frames").into_result(
                    ScanInputKind::Video,
                    frames,
                    Some(extracted.duration_ms),
                );
                if let Some(coverage) = &mut result.coverage {
                    coverage.preprocessing_version = video.config_fingerprint();
                }
                return Ok(result);
            }
            // Format comes from bytes, never from untrusted MIME metadata.
            let jpeg = normalize_image(&media.bytes)?;
            let result = self
                .client
                .moderate_guarded(ModerationInput::Image(&jpeg), deadline, guard)
                .await?;
            return Ok(result.into_result(ScanInputKind::Image, 1, None));
        }
        let text = request.text.as_deref().unwrap_or_default();
        if text.trim().is_empty() {
            let mut result = ProviderScanResult::completed(
                PROVIDER_NAME,
                SafetyProviderCapability::GeneralMediaModeration,
            );
            result.outcome = ScanOutcome::NoKnownMatch;
            return Ok(result);
        }
        let result = self
            .client
            .moderate_guarded(ModerationInput::Text(text), deadline, guard)
            .await?;
        Ok(result.into_result(ScanInputKind::Text, 0, None))
    }
}

#[async_trait]
impl SafetyProvider for OpenAiModerationProvider {
    fn supports_content_reuse(&self) -> bool {
        true
    }
    fn name(&self) -> &str {
        PROVIDER_NAME
    }
    fn capabilities(&self) -> &[SafetyProviderCapability] {
        &[SafetyProviderCapability::GeneralMediaModeration]
    }
    fn config_fingerprint(&self) -> String {
        format!(
            "{}|{}",
            self.client.config().fingerprint(),
            self.video
                .as_ref()
                .map(|v| v.config_fingerprint())
                .unwrap_or_else(|| "video-disabled".into())
        )
    }
    async fn scan(&self, request: &ProviderScanRequest) -> Result<ProviderScanResult, ScanError> {
        let _queue = self
            .queue
            .try_acquire()
            .map_err(|_| ScanError::Unavailable("moderation scan queue full".into()))?;
        let deadline = Instant::now() + self.client.config().scan_timeout;
        timeout_at(deadline, self.scan_inner(request, deadline, None))
            .await
            .map_err(|_| ScanError::Timeout("moderation scan deadline exceeded".into()))?
    }

    async fn scan_guarded(
        &self,
        request: &ProviderScanRequest,
        guard: &dyn kukuri_cn_safety::provider::ScanReferenceGuard,
    ) -> Result<ProviderScanResult, ScanError> {
        let _queue = self
            .queue
            .try_acquire()
            .map_err(|_| ScanError::Unavailable("moderation scan queue full".into()))?;
        let deadline = Instant::now() + self.client.config().scan_timeout;
        timeout_at(deadline, self.scan_inner(request, deadline, Some(guard)))
            .await
            .map_err(|_| ScanError::Timeout("moderation scan deadline exceeded".into()))?
    }
}

fn is_video(bytes: &[u8]) -> bool {
    (bytes.len() >= 12 && &bytes[4..8] == b"ftyp") || bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3])
}

fn normalize_image(bytes: &[u8]) -> Result<Vec<u8>, ScanError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| invalid("unrecognized moderation media format"))?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP | ImageFormat::Gif)
    ) {
        return Err(invalid("unsupported moderation image format"));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(3840);
    limits.max_image_height = Some(3840);
    limits.max_alloc = Some(64 * 1024 * 1024);
    if reader.format() == Some(ImageFormat::Png)
        && image::codecs::png::PngDecoder::new(Cursor::new(bytes))
            .map_err(|_| invalid("invalid PNG image"))?
            .is_apng()
            .map_err(|_| invalid("invalid PNG animation metadata"))?
    {
        return Err(invalid("animated images require a temporal scan"));
    }
    if reader.format() == Some(ImageFormat::WebP)
        && image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))
            .map_err(|_| invalid("invalid WebP image"))?
            .has_animation()
    {
        return Err(invalid("animated images require a temporal scan"));
    }
    let decoded = if reader.format() == Some(ImageFormat::Gif) {
        let mut decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
            .map_err(|_| invalid("invalid GIF image"))?;
        decoder
            .set_limits(limits)
            .map_err(|_| invalid("GIF exceeds image limits"))?;
        let mut frames = decoder.into_frames();
        let first = frames
            .next()
            .ok_or_else(|| invalid("GIF has no frame"))?
            .map_err(|_| invalid("GIF decode failed"))?;
        if frames.next().is_some() {
            return Err(invalid("animated images require a temporal scan"));
        }
        image::DynamicImage::ImageRgba8(first.into_buffer())
    } else {
        reader.limits(limits);
        reader
            .decode()
            .map_err(|_| invalid("moderation image decode failed"))?
    };
    let image = decoded.thumbnail(512, 512).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 75)
        .encode_image(&image)
        .map_err(|_| invalid("moderation image encode failed"))?;
    if jpeg.len() > 256 * 1024 {
        return Err(invalid("normalized moderation image too large"));
    }
    Ok(jpeg)
}
