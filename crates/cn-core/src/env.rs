//! community-node 共通の環境変数パースヘルパ。
//!
//! 複数の community-node サービス（`cn-user-api` / `cn-indexer` 等）が同じ env 記法（bool トークン、
//! CSV）を共有するため、ここを単一の真実源にして解釈の drift を防ぐ。

use anyhow::{Result, anyhow};

/// bool 環境変数をパースする。未設定 / 空文字は `default`。
///
/// 受理するトークン（大文字小文字無視）:
/// - true: `1` / `true` / `yes` / `on`
/// - false: `0` / `false` / `no` / `off`
pub fn parse_bool_env(var_name: &str, default: bool) -> Result<bool> {
    match std::env::var(var_name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "" => Ok(default),
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(anyhow!("failed to parse {var_name}: `{other}`")),
        },
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(anyhow!("{var_name}: {error}")),
    }
}

/// CSV 環境変数を trim + 空要素除去した Vec でパースする。未設定なら空 Vec。
pub fn parse_csv_env(var_name: &str) -> Vec<String> {
    std::env::var(var_name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|item| {
                    let trimmed = item.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    #[test]
    fn parse_bool_env_reads_tokens() {
        let _guard = env_lock();
        let key = "KUKURI_CN_TEST_BOOL_ENV";
        for (value, expected) in [
            ("1", true),
            ("true", true),
            ("YES", true),
            ("on", true),
            ("0", false),
            ("false", false),
            ("no", false),
            ("OFF", false),
        ] {
            unsafe { std::env::set_var(key, value) };
            assert_eq!(parse_bool_env(key, !expected).unwrap(), expected);
        }
        unsafe { std::env::set_var(key, "") };
        assert!(parse_bool_env(key, true).unwrap());
        unsafe { std::env::remove_var(key) };
        assert!(!parse_bool_env(key, false).unwrap());
        unsafe { std::env::set_var(key, "maybe") };
        assert!(parse_bool_env(key, false).is_err());
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn parse_csv_env_trims_and_drops_empty() {
        let _guard = env_lock();
        let key = "KUKURI_CN_TEST_CSV_ENV";
        unsafe { std::env::set_var(key, " a , ,b,, c ") };
        assert_eq!(parse_csv_env(key), vec!["a", "b", "c"]);
        unsafe { std::env::remove_var(key) };
        assert!(parse_csv_env(key).is_empty());
    }
}
