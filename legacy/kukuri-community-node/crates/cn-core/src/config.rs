use anyhow::{anyhow, Context, Result};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

pub fn required_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing env: {name}"))
}

pub fn socket_addr_from_env(name: &str, default: &str) -> Result<SocketAddr> {
    let value = env::var(name).unwrap_or_else(|_| default.to_string());
    SocketAddr::from_str(&value).map_err(|err| anyhow!("invalid socket addr for {name}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        key: &'static str,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            env::remove_var(self.key);
        }
    }

    fn set_env(key: &'static str, value: &str) -> EnvGuard {
        env::set_var(key, value);
        EnvGuard { key }
    }

    #[test]
    fn required_env_reads_value() {
        let _guard = set_env("CN_TEST_REQUIRED_ENV_PRESENT", "value");
        let value = required_env("CN_TEST_REQUIRED_ENV_PRESENT").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn required_env_missing_returns_error() {
        env::remove_var("CN_TEST_REQUIRED_ENV_MISSING");
        assert!(required_env("CN_TEST_REQUIRED_ENV_MISSING").is_err());
    }

    #[test]
    fn socket_addr_from_env_uses_default() {
        env::remove_var("CN_TEST_SOCKET_DEFAULT");
        let addr = socket_addr_from_env("CN_TEST_SOCKET_DEFAULT", "127.0.0.1:1234").unwrap();
        assert_eq!(addr, "127.0.0.1:1234".parse().unwrap());
    }

    #[test]
    fn socket_addr_from_env_parses_override() {
        let _guard = set_env("CN_TEST_SOCKET_OVERRIDE", "0.0.0.0:4321");
        let addr = socket_addr_from_env("CN_TEST_SOCKET_OVERRIDE", "127.0.0.1:1234").unwrap();
        assert_eq!(addr, "0.0.0.0:4321".parse().unwrap());
    }

    #[test]
    fn socket_addr_from_env_invalid_returns_error() {
        let _guard = set_env("CN_TEST_SOCKET_INVALID", "not-a-socket");
        assert!(socket_addr_from_env("CN_TEST_SOCKET_INVALID", "127.0.0.1:1234").is_err());
    }
}
