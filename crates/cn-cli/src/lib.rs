//! `cn-cli`のDB非依存な入力変換・表示helper。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

/// Epoch秒をRFC3339へ整形する。表現不能な値は従来どおり数値文字列へfallbackする。
pub fn format_timestamp(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| timestamp.to_string())
}

/// Epoch秒またはRFC3339を認証rollout/admission用のepoch秒へ変換する。
pub fn parse_enforce_at(value: &str) -> Result<i64> {
    if let Ok(timestamp) = value.parse::<i64>() {
        return Ok(timestamp);
    }
    let parsed = DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("failed to parse RFC3339 timestamp `{value}`"))?;
    Ok(parsed.with_timezone(&Utc).timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_enforce_at_accepts_epoch_boundaries() {
        assert_eq!(parse_enforce_at("0").unwrap(), 0);
        assert_eq!(parse_enforce_at("-1").unwrap(), -1);
        assert_eq!(parse_enforce_at(&i64::MAX.to_string()).unwrap(), i64::MAX);
    }

    #[test]
    fn parse_enforce_at_normalizes_rfc3339_offsets() {
        assert_eq!(
            parse_enforce_at("2026-07-11T23:30:00+09:00").unwrap(),
            parse_enforce_at("2026-07-11T14:30:00Z").unwrap()
        );
    }

    #[test]
    fn parse_enforce_at_rejects_blank_and_invalid_values() {
        for value in ["", "tomorrow", "2026-07-11"] {
            let error = parse_enforce_at(value).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("failed to parse RFC3339 timestamp")
            );
        }
    }

    #[test]
    fn format_timestamp_uses_rfc3339_or_numeric_fallback() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00+00:00");
        assert_eq!(format_timestamp(i64::MAX), i64::MAX.to_string());
    }

    #[test]
    fn parse_and_format_round_trip_representable_seconds() {
        let timestamp = parse_enforce_at("2026-07-11T14:30:00Z").unwrap();
        assert_eq!(
            parse_enforce_at(&format_timestamp(timestamp)).unwrap(),
            timestamp
        );
    }
}
