use anyhow::{anyhow, Context, Result};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;

pub fn required_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing env: {}", name))
}

pub fn socket_addr_from_env(name: &str, default: &str) -> Result<SocketAddr> {
    let value = env::var(name).unwrap_or_else(|_| default.to_string());
    SocketAddr::from_str(&value)
        .map_err(|err| anyhow!("invalid socket addr for {}: {}", name, err))
}
