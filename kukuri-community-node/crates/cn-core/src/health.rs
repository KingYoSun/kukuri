use anyhow::{anyhow, Result};
use reqwest::Client;
use std::collections::HashMap;

pub fn parse_health_targets(
    list_env: &str,
    fallback: &[(&str, &str, &str)],
) -> HashMap<String, String> {
    let mut targets = HashMap::new();
    if let Ok(raw) = std::env::var(list_env) {
        for entry in raw.split(',') {
            if let Some((name, url)) = entry.split_once('=') {
                if !name.trim().is_empty() && !url.trim().is_empty() {
                    targets.insert(name.trim().to_string(), url.trim().to_string());
                }
            }
        }
    }

    for (name, env, default_url) in fallback {
        if targets.contains_key(*name) {
            continue;
        }
        let value = std::env::var(env).unwrap_or_else(|_| (*default_url).to_string());
        targets.insert((*name).to_string(), value);
    }

    targets
}

pub async fn ensure_health_targets_ready(
    client: &Client,
    targets: &HashMap<String, String>,
) -> Result<()> {
    for (dependency, url) in targets {
        ensure_health_target_ready(client, dependency, url).await?;
    }
    Ok(())
}

pub async fn ensure_health_target_ready(
    client: &Client,
    dependency: &str,
    url: &str,
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| anyhow!("dependency `{dependency}` health request failed: {err}"))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "dependency `{dependency}` health returned status {}",
            response.status()
        ));
    }

    Ok(())
}

pub async fn ensure_endpoint_reachable(client: &Client, dependency: &str, url: &str) -> Result<()> {
    client
        .get(url)
        .send()
        .await
        .map_err(|err| anyhow!("dependency `{dependency}` request failed: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
    }

    fn set_env_var(key: &str, value: &str) {
        // SAFETY: tests serialize env access via a global mutex.
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env_var(key: &str) {
        // SAFETY: tests serialize env access via a global mutex.
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn parse_health_targets_prefers_explicit_map_and_fills_fallback() {
        let _guard = env_lock();
        set_env_var("TEST_HEALTH_TARGETS", "relay=http://relay:8082/healthz");
        set_env_var("TEST_USER_HEALTH_URL", "http://user-api:8080/healthz");

        let targets = parse_health_targets(
            "TEST_HEALTH_TARGETS",
            &[
                (
                    "relay",
                    "TEST_RELAY_HEALTH_URL",
                    "http://default-relay/healthz",
                ),
                (
                    "user-api",
                    "TEST_USER_HEALTH_URL",
                    "http://default-user/healthz",
                ),
            ],
        );

        assert_eq!(
            targets.get("relay"),
            Some(&"http://relay:8082/healthz".to_string())
        );
        assert_eq!(
            targets.get("user-api"),
            Some(&"http://user-api:8080/healthz".to_string())
        );

        remove_env_var("TEST_HEALTH_TARGETS");
        remove_env_var("TEST_USER_HEALTH_URL");
    }
}
