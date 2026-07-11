//! Community-node binaryに共通する軽量なruntime初期化helper。

use tracing_subscriber::EnvFilter;

/// `RUST_LOG`を優先し、未設定または不正ならbinary固有の既定filterでtracingを初期化する。
///
/// testや同一process内の複数runtimeから呼ばれても、既にsubscriberがある場合は成功扱いにする。
pub fn init_tracing(default_filter: &str) {
    let env_filter = select_env_filter(std::env::var("RUST_LOG").ok().as_deref(), default_filter);
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .try_init();
}

fn select_env_filter(rust_log: Option<&str>, default_filter: &str) -> EnvFilter {
    rust_log
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new(default_filter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_log_overrides_binary_default() {
        let filter = select_env_filter(Some("warn,my_crate=trace"), "info,binary=debug");
        assert_eq!(
            filter.to_string(),
            EnvFilter::new("warn,my_crate=trace").to_string()
        );
    }

    #[test]
    fn missing_rust_log_uses_binary_default() {
        let filter = select_env_filter(None, "info,binary=debug");
        assert_eq!(
            filter.to_string(),
            EnvFilter::new("info,binary=debug").to_string()
        );
    }

    #[test]
    fn invalid_rust_log_uses_binary_default() {
        let filter = select_env_filter(Some("[invalid"), "info,binary=debug");
        assert_eq!(
            filter.to_string(),
            EnvFilter::new("info,binary=debug").to_string()
        );
    }
}
