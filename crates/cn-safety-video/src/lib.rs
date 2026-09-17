//! Bounded CN video extraction. Decoder bytes and files remain transient.
mod config;
mod probe;
mod process;
mod sandbox;
mod workdir;

use async_trait::async_trait;
use config::invalid;
pub use config::{EXTRACTOR_VERSION, VideoExtractConfig};
use kukuri_cn_safety::{
    FetchedMedia, ScanError,
    provider::{VideoFrameExtractor, VideoScanFrames},
};
use sha2::{Digest, Sha256};
use std::{ffi::OsString, sync::Arc, time::Duration};
use tokio::{
    sync::{Semaphore, watch},
    time::Instant,
};

pub struct FfmpegVideoExtractor {
    config: VideoExtractConfig,
    fingerprint: String,
    active: Arc<Semaphore>,
}

impl FfmpegVideoExtractor {
    /// Decode only bundled synthetic fixtures; readiness never retrieves user content.
    pub async fn readiness_probe(&self) -> Result<FetchedMedia, ScanError> {
        let mp4 = self.extract(include_bytes!("probes/benign.mp4")).await?;
        let webm = self.extract(include_bytes!("probes/benign.webm")).await?;
        if mp4.frames.len() != 1 || webm.frames.len() != 1 {
            return Err(invalid("decoder probe frame count mismatch"));
        }
        mp4.frames
            .into_iter()
            .next()
            .ok_or_else(|| invalid("decoder probe has no frame"))
    }
    pub fn new(config: VideoExtractConfig) -> Result<Self, ScanError> {
        config.validate()?;
        let mut digest = Sha256::new();
        digest.update(EXTRACTOR_VERSION);
        digest.update(serde_json::to_vec(&config).expect("serializable video config"));
        for path in [&config.ffmpeg, &config.ffprobe] {
            let metadata = std::fs::metadata(path)
                .map_err(|_| ScanError::Unavailable("video decoder executable missing".into()))?;
            if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
                return Err(invalid("invalid decoder executable"));
            }
            digest.update(
                std::fs::read(path)
                    .map_err(|_| ScanError::Unavailable("cannot identify video decoder".into()))?,
            );
        }
        // Fail startup instead of using a durable filesystem as a fallback.
        drop(workdir::JobDirectory::create(&config.temporary_root)?);
        Ok(Self {
            config,
            fingerprint: format!("{EXTRACTOR_VERSION}:{}", hex::encode(digest.finalize())),
            active: Arc::new(Semaphore::new(1)),
        })
    }
}

struct CancelOnDrop(watch::Sender<bool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(true);
    }
}

#[async_trait]
impl VideoFrameExtractor for FfmpegVideoExtractor {
    fn config_fingerprint(&self) -> String {
        self.fingerprint.clone()
    }
    async fn extract(&self, bytes: &[u8]) -> Result<VideoScanFrames, ScanError> {
        if bytes.is_empty() || bytes.len() > self.config.max_input_bytes {
            return Err(invalid("video input exceeds bounds"));
        }
        // Take the permit before copying input. The supervisor retains it until cleanup.
        let permit =
            tokio::time::timeout(Duration::from_secs(60), self.active.clone().acquire_owned())
                .await
                .map_err(|_| ScanError::Timeout("video extraction queue wait exceeded".into()))?
                .map_err(|_| ScanError::Unavailable("video extractor is stopping".into()))?;
        let input = bytes.to_vec();
        let config = self.config.clone();
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let _cancel = CancelOnDrop(cancel_tx);
        // Detaching on caller cancellation is intentional: this owner kills/reaps the child
        // and drops the temporary directory before admitting the next extraction.
        let worker = tokio::spawn(async move {
            let _permit = permit;
            extract_job(&config, &input, &mut cancel_rx).await
        });
        worker
            .await
            .map_err(|_| ScanError::Unavailable("video extraction supervisor failed".into()))?
    }
}

async fn extract_job(
    config: &VideoExtractConfig,
    bytes: &[u8],
    cancel: &mut watch::Receiver<bool>,
) -> Result<VideoScanFrames, ScanError> {
    let job = workdir::JobDirectory::create(&config.temporary_root)?;
    let input = job.path().join("input.media");
    tokio::fs::write(&input, bytes)
        .await
        .map_err(|_| invalid("cannot stage bounded video input"))?;
    let mp4 = bytes.len() >= 12 && &bytes[4..8] == b"ftyp";
    let webm = bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]);
    if !mp4 && !webm {
        return Err(invalid("unsupported video signature"));
    }
    let demuxer = if mp4 { "mov" } else { "matroska" };
    let mut args = args_of(&[
        "-v",
        "error",
        "-threads",
        "1",
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        demuxer,
        "-f",
        demuxer,
    ]);
    if mp4 {
        args.extend(args_of(&["-enable_drefs", "0", "-use_absolute_path", "0"]));
    }
    args.extend(args_of(&["-show_streams", "-show_format", "-of", "json"]));
    args.push(input.as_os_str().into());
    let output = process::run(
        &config.ffprobe,
        &args,
        job.path(),
        config.max_frame_bytes,
        Instant::now() + config.probe_timeout,
        cancel,
    )
    .await?;
    let probe = probe::VideoProbe::parse(&output)?;
    let samples = config.sample_times_us(probe.duration_us)?;
    let count = samples.len();
    // fps with one interval of final-frame padding samples the display timeline,
    // including a single-frame video and a final frame displayed across a VFR gap.
    let interval = probe.duration_us as f64 / (count as f64 * 1_000_000.0);
    let vf = format!(
        "setpts=PTS-STARTPTS-{:.6}/TB,tpad=stop_mode=clone:stop_duration={interval:.6},fps=fps={}/{}:start_time=0:round=up,scale=w='min(512,iw)':h='min(512,ih)':force_original_aspect_ratio=decrease",
        samples[0] as f64 / 1_000_000.0,
        count * 1_000_000,
        probe.duration_us
    );
    let mut args = args_of(&[
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-xerror",
        "-err_detect",
        "explode",
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        demuxer,
        "-f",
        demuxer,
    ]);
    if mp4 {
        args.extend(args_of(&["-enable_drefs", "0", "-use_absolute_path", "0"]));
    }
    // Input and output codec options have separate scopes in FFmpeg. Bound the
    // decoder before -i as well as the JPEG encoder below, regardless of CPU count.
    args.extend(args_of(&["-threads", "1"]));
    args.push("-i".into());
    args.push(input.as_os_str().into());
    args.extend(args_of(&[
        "-map",
        &format!("0:{}", probe.stream),
        "-an",
        "-sn",
        "-dn",
        "-vf",
        &vf,
        "-frames:v",
        &count.to_string(),
        "-threads",
        "1",
        "-filter_threads",
        "1",
        "-q:v",
        "4",
        "-f",
        "image2",
        "-y",
    ]));
    args.push(job.path().join("frame-%02d.jpg").as_os_str().into());
    process::run(
        &config.ffmpeg,
        &args,
        job.path(),
        config.max_frame_bytes,
        Instant::now() + config.decode_timeout,
        cancel,
    )
    .await?;
    let mut frames = Vec::with_capacity(count);
    let mut total = 0usize;
    for index in 1..=count {
        let file = job.path().join(format!("frame-{index:02}.jpg"));
        let metadata = tokio::fs::metadata(&file)
            .await
            .map_err(|_| invalid("video extraction returned fewer frames than planned"))?;
        if metadata.len() == 0 || metadata.len() > config.max_frame_bytes as u64 {
            return Err(invalid("extracted frame exceeds bounds"));
        }
        total = total.saturating_add(metadata.len() as usize);
        if total > config.max_output_bytes {
            return Err(invalid("video frame total exceeds bounds"));
        }
        let bytes = tokio::fs::read(file)
            .await
            .map_err(|_| invalid("cannot read extracted frame"))?;
        if !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            return Err(invalid("extracted frame is not JPEG"));
        }
        frames.push(FetchedMedia {
            bytes,
            content_type: "image/jpeg".into(),
        });
    }
    Ok(VideoScanFrames {
        frames,
        duration_ms: probe.duration_us.div_ceil(1000),
    })
}

fn args_of(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}
