use crate::config::invalid;
use kukuri_cn_safety::ScanError;
use serde::Deserialize;

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
    format: Format,
}
#[derive(Deserialize)]
struct Stream {
    index: u32,
    codec_type: String,
    #[serde(default)]
    codec_name: String,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<String>,
    #[serde(default)]
    disposition: Disposition,
}
#[derive(Default, Deserialize)]
struct Disposition {
    #[serde(default)]
    attached_pic: u8,
}
#[derive(Deserialize)]
struct Format {
    format_name: String,
    duration: Option<String>,
}

pub struct VideoProbe {
    pub stream: u32,
    pub duration_us: u64,
}

impl VideoProbe {
    pub fn parse(bytes: &[u8]) -> Result<Self, ScanError> {
        let probe: Probe =
            serde_json::from_slice(bytes).map_err(|_| invalid("invalid video probe response"))?;
        let mp4 = probe
            .format
            .format_name
            .split(',')
            .any(|format| format == "mp4");
        let webm = probe
            .format
            .format_name
            .split(',')
            .any(|format| format == "webm");
        if !mp4 && !webm {
            return Err(invalid("unsupported video container"));
        }
        let stream = probe
            .streams
            .iter()
            .find(|s| s.codec_type == "video" && s.disposition.attached_pic == 0)
            .ok_or_else(|| invalid("video has no visual stream"))?;
        if !((mp4 && stream.codec_name == "h264")
            || (webm && matches!(stream.codec_name.as_str(), "vp8" | "vp9")))
        {
            return Err(invalid("unsupported video codec"));
        }
        let (width, height) = (stream.width.unwrap_or(0), stream.height.unwrap_or(0));
        if width == 0 || height == 0 || width.max(height) > 3840 || width.min(height) > 2160 {
            return Err(invalid("unsupported video dimensions"));
        }
        let duration = stream
            .duration
            .as_deref()
            .filter(|s| *s != "N/A")
            .or(probe.format.duration.as_deref())
            .ok_or_else(|| invalid("video duration is missing"))?;
        let seconds: f64 = duration
            .parse()
            .map_err(|_| invalid("invalid video duration"))?;
        if !seconds.is_finite() || seconds <= 0.0 || seconds > 600.0 {
            return Err(invalid("unsupported video duration"));
        }
        let duration_us = (seconds * 1_000_000.0).round() as u64;
        if duration_us == 0 {
            return Err(invalid("empty video duration"));
        }
        Ok(Self {
            stream: stream.index,
            duration_us,
        })
    }
}
