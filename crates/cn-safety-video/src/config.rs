use kukuri_cn_safety::ScanError;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

pub const EXTRACTOR_VERSION: &str = "video-midpoints-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoExtractConfig {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub temporary_root: PathBuf,
    pub decoder_build_id: String,
    pub max_input_bytes: usize,
    pub max_duration_us: u64,
    pub interval_us: u64,
    pub max_frames: usize,
    pub max_frame_bytes: usize,
    pub max_output_bytes: usize,
    pub probe_timeout: Duration,
    pub decode_timeout: Duration,
}

impl Default for VideoExtractConfig {
    fn default() -> Self {
        Self {
            ffmpeg: "/usr/bin/ffmpeg".into(),
            ffprobe: "/usr/bin/ffprobe".into(),
            temporary_root: "/dev/shm/kukuri-video".into(),
            decoder_build_id: "local".into(),
            max_input_bytes: 32 * 1024 * 1024,
            max_duration_us: 600_000_000,
            interval_us: 5_000_000,
            max_frames: 8,
            max_frame_bytes: 256 * 1024,
            max_output_bytes: 2 * 1024 * 1024,
            probe_timeout: Duration::from_secs(5),
            decode_timeout: Duration::from_secs(30),
        }
    }
}

impl VideoExtractConfig {
    pub fn from_env() -> Result<Self, ScanError> {
        let mut config = Self::default();
        for (name, target) in [
            ("COMMUNITY_NODE_VIDEO_FFMPEG", &mut config.ffmpeg),
            ("COMMUNITY_NODE_VIDEO_FFPROBE", &mut config.ffprobe),
            ("COMMUNITY_NODE_VIDEO_TMPDIR", &mut config.temporary_root),
        ] {
            if let Some(value) = std::env::var_os(name) {
                *target = value.into();
            }
        }
        config.decoder_build_id = std::env::var("COMMUNITY_NODE_VIDEO_DECODER_BUILD_ID")
            .or_else(|_| std::fs::read_to_string("/usr/local/share/kukuri/decoder-build-id"))
            .unwrap_or_else(|_| "local".into());
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ScanError> {
        if !self.ffmpeg.is_absolute()
            || !self.ffprobe.is_absolute()
            || !self.temporary_root.is_absolute()
            || self.decoder_build_id.trim().is_empty()
            || self.max_input_bytes == 0
            || self.max_input_bytes > 32 * 1024 * 1024
            || self.max_duration_us == 0
            || self.max_duration_us > 600_000_000
            || self.interval_us == 0
            || self.max_frames == 0
            || self.max_frames > 8
            || self.max_frame_bytes == 0
            || self.max_frame_bytes > 256 * 1024
            || self.max_output_bytes == 0
            || self.max_output_bytes > 2 * 1024 * 1024
            || self.probe_timeout.is_zero()
            || self.probe_timeout > Duration::from_secs(5)
            || self.decode_timeout.is_zero()
            || self.decode_timeout > Duration::from_secs(30)
        {
            return Err(invalid("invalid video extraction bounds"));
        }
        Ok(())
    }

    pub fn sample_times_us(&self, duration_us: u64) -> Result<Vec<u64>, ScanError> {
        self.validate()?;
        if duration_us == 0 || duration_us > self.max_duration_us {
            return Err(invalid("invalid video duration"));
        }
        let count = duration_us
            .div_ceil(self.interval_us)
            .min(self.max_frames as u64)
            .max(1);
        Ok((0..count)
            .map(|i| duration_us * (2 * i + 1) / (2 * count))
            .collect())
    }
}

pub(crate) fn invalid(message: &str) -> ScanError {
    ScanError::Protocol(message.to_owned())
}
