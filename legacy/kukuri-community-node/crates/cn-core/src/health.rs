use anyhow::{anyhow, Result};
use reqwest::Client;
use std::collections::HashMap;

pub fn parse_health_targets(
    list_env: &str,
    fallback: &[(&str, &str, &str)],
) -> HashMap<String, String> {
    parse_health_targets_with(list_env, fallback, |key| std::env::var(key).ok())
}

fn parse_health_targets_with<F>(
    list_env: &str,
    fallback: &[(&str, &str, &str)],
    mut get_env: F,
) -> HashMap<String, String>
where
    F: FnMut(&str) -> Option<String>,
{
    let mut targets = HashMap::new();
    if let Some(raw) = get_env(list_env) {
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
        let value = get_env(env).unwrap_or_else(|| (*default_url).to_string());
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

    #[test]
    fn parse_health_targets_prefers_explicit_map_and_fills_fallback() {
        let env = HashMap::from([
            (
                "TEST_HEALTH_TARGETS".to_string(),
                "relay=http://relay:8082/healthz".to_string(),
            ),
            (
                "TEST_USER_HEALTH_URL".to_string(),
                "http://user-api:8080/healthz".to_string(),
            ),
        ]);

        let targets = parse_health_targets_with(
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
            |key| env.get(key).cloned(),
        );

        assert_eq!(
            targets.get("relay"),
            Some(&"http://relay:8082/healthz".to_string())
        );
        assert_eq!(
            targets.get("user-api"),
            Some(&"http://user-api:8080/healthz".to_string())
        );
    }
}
