use anyhow::{anyhow, Result};
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone)]
pub struct MeiliClient {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl MeiliClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Result<Self> {
        let trimmed = base_url.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("MEILI_URL is empty"));
        }
        Ok(Self {
            base_url: trimmed.trim_end_matches('/').to_string(),
            api_key: api_key.filter(|value| !value.trim().is_empty()),
            http: reqwest::Client::new(),
        })
    }

    pub async fn ensure_index(
        &self,
        uid: &str,
        primary_key: &str,
        settings: Option<Value>,
    ) -> Result<()> {
        let resp = self
            .request(Method::GET, &format!("/indexes/{uid}"))
            .send()
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            let create_resp = self
                .request(Method::POST, "/indexes")
                .json(&json!({ "uid": uid, "primaryKey": primary_key }))
                .send()
                .await?;
            ensure_success(create_resp).await?;
            if let Some(settings) = settings {
                self.update_settings(uid, settings).await?;
            }
            return Ok(());
        }
        ensure_success(resp).await?;
        Ok(())
    }

    pub async fn update_settings(&self, uid: &str, settings: Value) -> Result<()> {
        let resp = self
            .request(Method::PATCH, &format!("/indexes/{uid}/settings"))
            .json(&settings)
            .send()
            .await?;
        ensure_success(resp).await
    }

    pub async fn upsert_documents<T: Serialize>(&self, uid: &str, docs: &[T]) -> Result<()> {
        if docs.is_empty() {
            return Ok(());
        }
        let resp = self
            .request(Method::POST, &format!("/indexes/{uid}/documents"))
            .query(&[("primaryKey", "event_id")])
            .json(docs)
            .send()
            .await?;
        ensure_success(resp).await
    }

    pub async fn delete_documents(&self, uid: &str, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let resp = self
            .request(
                Method::POST,
                &format!("/indexes/{uid}/documents/delete-batch"),
            )
            .json(ids)
            .send()
            .await?;
        ensure_success(resp).await
    }

    pub async fn delete_document(&self, uid: &str, id: &str) -> Result<()> {
        let resp = self
            .request(Method::DELETE, &format!("/indexes/{uid}/documents/{id}"))
            .send()
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }
        ensure_success(resp).await
    }

    pub async fn delete_all_documents(&self, uid: &str) -> Result<()> {
        let resp = self
            .request(
                Method::POST,
                &format!("/indexes/{uid}/documents/delete-all"),
            )
            .send()
            .await?;
        ensure_success(resp).await
    }

    pub async fn search(
        &self,
        uid: &str,
        query: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Value> {
        let resp = self
            .request(Method::POST, &format!("/indexes/{uid}/search"))
            .json(&json!({
                "q": query,
                "limit": limit,
                "offset": offset
            }))
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !is_success(status) {
            return Err(anyhow!("meilisearch error: {} - {}", status, body));
        }
        let parsed: Value = serde_json::from_str(&body)?;
        Ok(parsed)
    }

    pub async fn check_ready(&self) -> Result<()> {
        let response = self.request(Method::GET, "/health").send().await?;
        ensure_success(response).await
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let builder = self.http.request(method, url);
        if let Some(key) = &self.api_key {
            builder
                .header("Authorization", format!("Bearer {key}"))
                .header("X-Meili-API-Key", key)
        } else {
            builder
        }
    }
}

pub fn topic_index_uid(topic_id: &str) -> String {
    let mut uid = String::from("topic_");
    for ch in topic_id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            uid.push(ch.to_ascii_lowercase());
        } else {
            uid.push('_');
        }
    }
    uid
}

async fn ensure_success(resp: reqwest::Response) -> Result<()> {
    let status = resp.status();
    if is_success(status) {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    Err(anyhow!("meilisearch error: {} - {}", status, body))
}

fn is_success(status: StatusCode) -> bool {
    status.is_success() || status == StatusCode::ACCEPTED
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_index_uid_sanitizes_characters() {
        assert_eq!(topic_index_uid("kukuri:abcDEF"), "topic_kukuri_abcdef");
        assert_eq!(
            topic_index_uid("kukuri:topic/one"),
            "topic_kukuri_topic_one"
        );
    }
}
