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
}

impl DriftReport {
    pub fn is_clean(&self) -> bool {
        self.changed.is_empty() && self.missing.is_empty() && self.unexpected.is_empty()
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
        s.push_str("`generate-docs` で再生成してください。");
        s
    }
}

/// config から再生成した内容と `dir` 配下の内容を比較する。
pub fn check_drift(config: &ResolvedConfig, dir: &Path) -> Result<DriftReport> {
    let expected = generate_all(config);
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
    Ok(report)
}
