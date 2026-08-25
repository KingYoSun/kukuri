//! 生成済み文書と現在の config から再生成した結果の drift 検出。
//!
//! `check-disclosures` は、config から文書を再生成し、出力ディレクトリの内容と比較する。
//! 差分があれば non-zero exit する想定。

use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::config::ResolvedConfig;
use crate::docs::generate_all;

/// drift の検出結果。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DriftReport {
    /// 内容が一致しないファイル。
    pub changed: Vec<String>,
    /// 出力ディレクトリに存在しない（未生成の）ファイル。
    pub missing: Vec<String>,
    /// 出力ディレクトリにあるが生成対象でない余分なファイル。
    pub unexpected: Vec<String>,
    /// config 由来の secret ID または private endpoint を含むファイル。
    pub sensitive: Vec<String>,
}

impl DriftReport {
    pub fn is_clean(&self) -> bool {
        self.changed.is_empty()
            && self.missing.is_empty()
            && self.unexpected.is_empty()
            && self.sensitive.is_empty()
    }

    /// 人間可読なサマリ。
    pub fn summary(&self) -> String {
        if self.is_clean() {
            return "生成文書は config と一致しています（drift なし）。".to_string();
        }
        let mut s = String::new();
        if !self.missing.is_empty() {
            s.push_str(&format!("未生成: {}\n", self.missing.join(", ")));
        }
        if !self.changed.is_empty() {
            s.push_str(&format!("差分あり: {}\n", self.changed.join(", ")));
        }
        if !self.unexpected.is_empty() {
            s.push_str(&format!("余分なファイル: {}\n", self.unexpected.join(", ")));
        }
        if !self.sensitive.is_empty() {
            s.push_str(&format!(
                "機密識別子・内部 endpoint の混入: {}\n",
                self.sensitive.join(", ")
            ));
        }
        s.push_str("`generate-docs` で再生成してください。");
        s
    }
}

/// config から再生成した内容と `dir` 配下の内容を比較する。
pub fn check_drift(config: &ResolvedConfig, dir: &Path) -> Result<DriftReport> {
    let expected = generate_all(config);
    let forbidden = forbidden_disclosure_values(config);
    let mut report = DriftReport::default();

    let mut expected_names: Vec<String> = Vec::new();
    for file in &expected {
        expected_names.push(file.filename.clone());
        let path = dir.join(&file.filename);
        match fs::read_to_string(&path) {
            Ok(actual) => {
                if actual != file.content {
                    report.changed.push(file.filename.clone());
                }
                if contains_sensitive_value(&actual, &forbidden) {
                    report.sensitive.push(file.filename.clone());
                }
            }
            Err(_) => report.missing.push(file.filename.clone()),
        }
    }

    // 余分なファイル（生成対象でない既知拡張子）の検出。
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_target = name.ends_with(".md") || name.ends_with(".json");
            if is_target && !expected_names.contains(&name) {
                report.unexpected.push(name);
            }
        }
    }

    report.changed.sort();
    report.missing.sort();
    report.unexpected.sort();
    report.sensitive.sort();
    report.sensitive.dedup();
    Ok(report)
}

fn forbidden_disclosure_values(config: &ResolvedConfig) -> Vec<String> {
    let mut values = vec![
        "127.0.0.1".to_string(),
        "localhost".to_string(),
        "0.0.0.0".to_string(),
    ];
    if let Some(deploy) = config.deploy() {
        values.extend(
            [
                Some(deploy.jwt_secret_id.clone()),
                Some(deploy.postgres_password_secret_id.clone()),
                deploy.channel_secret_key_secret_id.clone(),
                deploy.legal_data_key_secret_id.clone(),
                deploy.arcadedb_password_secret_id.clone(),
                deploy.arachnid_username_secret_id.clone(),
                deploy.arachnid_password_secret_id.clone(),
                deploy.vlm_api_key_secret_id.clone(),
                deploy.vlm_api_base_url.clone(),
            ]
            .into_iter()
            .flatten(),
        );
        values.extend(deploy.indexer_external_relay_urls.iter().cloned());
    }
    values.retain(|value| value.trim().len() >= 4);
    values.sort();
    values.dedup();
    values
}

fn contains_sensitive_value(content: &str, forbidden: &[String]) -> bool {
    let lower = content.to_ascii_lowercase();
    forbidden
        .iter()
        .any(|value| lower.contains(&value.to_ascii_lowercase()))
        || lower
            .split(|c: char| c.is_whitespace() || matches!(c, '/' | ':' | '(' | ')' | ','))
            .any(is_private_ipv4)
}

fn is_private_ipv4(token: &str) -> bool {
    let token = token.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
    let octets = token
        .split('.')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(octets) = octets else { return false };
    if octets.len() != 4 {
        return false;
    }
    octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
}
