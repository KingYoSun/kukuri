//! Secret-free failure classes for operator diagnostics. Messages are fixed strings owned here.
use kukuri_cn_safety::ScanError;

pub(crate) const QUEUE_TIMEOUT: &str = "video extraction queue wait exceeded";
pub(crate) const WARM_UP_TIMEOUT: &str = "video decoder warm-up deadline exceeded";
pub(crate) const PROBE_TIMEOUT: &str = "video probe deadline exceeded";
pub(crate) const DECODE_TIMEOUT: &str = "video decoder deadline exceeded";
pub(crate) const SCRATCH_BUSY: &str = "video tmpfs maintenance is busy";
pub(crate) const SPAWN_FAILED: &str = "cannot start sandboxed decoder";
pub(crate) const DECODER_EXITED: &str = "decoder failed or exceeded resource limits";
pub(crate) const CANCELLED: &str = "video scan cancelled";
pub(crate) const SCRATCH_UNAVAILABLE: &[&str] = &[
    "cannot create video tmpfs directory",
    "video scratch directory is not a private tmpfs",
    "cannot open video job lease",
    "cannot inspect video scratch directory",
    "cannot inspect video scratch entry",
    "cannot inspect video scratch type",
    "cannot clean abandoned video job",
    "cannot create video job directory",
    "cannot lock video job directory",
    "cannot stage bounded video input",
];
pub(crate) const DECODER_MISSING: &[&str] = &[
    "video decoder executable missing",
    "cannot identify video decoder",
    "invalid decoder executable",
];
pub(crate) const INVALID_CONFIG: &[&str] = &[
    "invalid video decoder paths or identity",
    "invalid video extraction bounds",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecoderFailure {
    InvalidConfig,
    DecoderMissing,
    ScratchBusy,
    ScratchUnavailable,
    QueueTimeout,
    WarmUpTimeout,
    ProbeTimeout,
    DecodeTimeout,
    SpawnFailed,
    DecoderExited,
    /// Input, probe output or extracted frames failed validation.
    Rejected,
    Cancelled,
    Internal,
}

pub fn classify_failure(error: &ScanError) -> DecoderFailure {
    let (ScanError::Unavailable(message)
    | ScanError::Timeout(message)
    | ScanError::Protocol(message)) = error;
    match message.as_str() {
        QUEUE_TIMEOUT => DecoderFailure::QueueTimeout,
        WARM_UP_TIMEOUT => DecoderFailure::WarmUpTimeout,
        PROBE_TIMEOUT => DecoderFailure::ProbeTimeout,
        DECODE_TIMEOUT => DecoderFailure::DecodeTimeout,
        SCRATCH_BUSY => DecoderFailure::ScratchBusy,
        SPAWN_FAILED => DecoderFailure::SpawnFailed,
        DECODER_EXITED => DecoderFailure::DecoderExited,
        CANCELLED => DecoderFailure::Cancelled,
        m if SCRATCH_UNAVAILABLE.contains(&m) => DecoderFailure::ScratchUnavailable,
        m if DECODER_MISSING.contains(&m) => DecoderFailure::DecoderMissing,
        m if INVALID_CONFIG.contains(&m) => DecoderFailure::InvalidConfig,
        "decoder wait failed" | "decoder output read failed" => DecoderFailure::Internal,
        _ if matches!(error, ScanError::Protocol(_)) => DecoderFailure::Rejected,
        _ => DecoderFailure::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_owned_message_has_a_specific_class() {
        let cases = [
            (
                ScanError::Timeout(QUEUE_TIMEOUT.into()),
                DecoderFailure::QueueTimeout,
            ),
            (
                ScanError::Timeout(WARM_UP_TIMEOUT.into()),
                DecoderFailure::WarmUpTimeout,
            ),
            (
                ScanError::Timeout(PROBE_TIMEOUT.into()),
                DecoderFailure::ProbeTimeout,
            ),
            (
                ScanError::Timeout(DECODE_TIMEOUT.into()),
                DecoderFailure::DecodeTimeout,
            ),
            (
                ScanError::Unavailable(SCRATCH_BUSY.into()),
                DecoderFailure::ScratchBusy,
            ),
            (
                ScanError::Unavailable(SPAWN_FAILED.into()),
                DecoderFailure::SpawnFailed,
            ),
            (
                ScanError::Unavailable(CANCELLED.into()),
                DecoderFailure::Cancelled,
            ),
            (
                ScanError::Protocol(DECODER_EXITED.into()),
                DecoderFailure::DecoderExited,
            ),
            (
                ScanError::Protocol("unsupported video codec".into()),
                DecoderFailure::Rejected,
            ),
            (
                ScanError::Unavailable("video extraction supervisor failed".into()),
                DecoderFailure::Internal,
            ),
        ];
        for (error, class) in cases {
            assert_eq!(classify_failure(&error), class, "{error}");
        }
        for message in SCRATCH_UNAVAILABLE {
            let error = if *message == "cannot stage bounded video input" {
                ScanError::Protocol((*message).into())
            } else {
                ScanError::Unavailable((*message).into())
            };
            assert_eq!(classify_failure(&error), DecoderFailure::ScratchUnavailable);
        }
        for message in DECODER_MISSING {
            let error = if *message == "invalid decoder executable" {
                ScanError::Protocol((*message).into())
            } else {
                ScanError::Unavailable((*message).into())
            };
            assert_eq!(classify_failure(&error), DecoderFailure::DecoderMissing);
        }
        for message in INVALID_CONFIG {
            let error = ScanError::Protocol((*message).into());
            assert_eq!(classify_failure(&error), DecoderFailure::InvalidConfig);
        }
    }
}
