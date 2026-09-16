//! 文書生成 test の共有 fixture（`generate.rs` / `generate_moderation_policy.rs`）。
#![allow(dead_code)]

pub fn doc(files: &[kukuri_cn_operator::GeneratedFile], name: &str) -> String {
    files
        .iter()
        .find(|f| f.filename == name)
        .unwrap_or_else(|| panic!("missing {name}"))
        .content
        .clone()
}

pub fn config_with_safety_providers(vlm_hosting_line: &str) -> String {
    let base = r#"server:
  domain: example-kukuri.net
  operator_name: Example Operator
  country: JP
  node_id: 79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
features:
  moderation: true
retention:
  connection_logs_days: 30
  moderation_logs_days: 180
safety:
  profile: public-node
  policy_version: 2026-06-public-node-v1
  providers:
    known_csam:
      provider: project-arachnid-shield
      required: true
      credential_secret_id: kukuri-cn-safety-known-csam
    general:
      provider: openai-compatible-vlm
"#;
    format!("{base}{vlm_hosting_line}")
}
