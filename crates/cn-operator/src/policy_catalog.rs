//! 公開法務文書カタログ。
//!
//! 文書本文、manifest、DB 同期へ渡す法務文書列の唯一の生成入口を提供する。

use serde_json::json;

use crate::config::{LegalDocumentKind, ResolvedConfig};
use crate::docs::{GeneratedFile, generate_all};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedLegalDocument {
    pub kind: LegalDocumentKind,
    pub slug: String,
    pub version: i32,
    pub effective_date: String,
    pub language: String,
    pub required: bool,
    pub title: String,
    pub filename: String,
    pub public_path: String,
    pub content: String,
    pub policy_snapshot_revision: String,
    pub authoritative_language: String,
    pub reference_translation: bool,
    pub translation_revision: Option<i32>,
    pub translation_of_version: Option<i32>,
}

/// 表示上の Markdown ではなく、法務上意味のある構造化入力だけを hash 化する。
pub fn policy_snapshot_revision(config: &ResolvedConfig) -> Option<String> {
    let legal = config.raw.legal.as_ref()?;
    let documents = legal
        .documents
        .iter()
        .map(|document| {
            json!({
                "kind": document.kind,
                "slug": document.slug,
                "version": document.version,
                "effective_date": document.effective_date,
                "language": document.language,
                "required": document.required,
                "supplemental_markdown": document.supplemental_markdown,
            })
        })
        .collect::<Vec<_>>();
    let input = json!({
        "schema_version": 1,
        "operator": {
            "domain": config.raw.server.domain,
            "operator_name": config.raw.server.operator_name,
            "country": config.raw.server.country,
            "contact": config.contact(),
            "identity_disclosure_request": legal.identity_disclosure_request,
        },
        "enabled_capabilities": config
            .enabled_capabilities()
            .into_iter()
            .map(|capability| json!({
                "key": capability.key(),
                "descriptor": capability.policy_descriptor(),
            }))
            .collect::<Vec<_>>(),
        "retention": config.raw.retention,
        "safety": config.raw.safety,
        "authority_scope": config.raw.manifest.authority_scope,
        "rights_request_initial_response_target_days":
            config.raw.manifest.rights_request_initial_response_target_days,
        "documents": documents,
    });
    let bytes = serde_json::to_vec(&input).expect("policy snapshot input serializes");
    Some(blake3::hash(&bytes).to_hex().to_string())
}

/// 公開法務文書を config の版情報と生成本文を結合して返す。
pub fn generate_legal_documents(config: &ResolvedConfig) -> Vec<GeneratedLegalDocument> {
    let files = generate_all(config);
    generate_legal_documents_from_files(config, &files)
}

/// 既に一度生成した文書列から policy catalog を組み立てる。server 起動時に
/// manifest/public disclosure/DB sync のため同じ本文を再生成しないための入口。
pub fn generate_legal_documents_from_files(
    config: &ResolvedConfig,
    files: &[GeneratedFile],
) -> Vec<GeneratedLegalDocument> {
    let Some(legal) = config.raw.legal.as_ref() else {
        return Vec::new();
    };
    let snapshot_revision =
        policy_snapshot_revision(config).expect("legal config has snapshot revision");
    let mut generated = Vec::new();
    for document in &legal.documents {
        let file = files
            .iter()
            .find(|file| file.filename == document.kind.filename());
        let Some(file) = file else {
            continue;
        };
        let mut content = file.content.clone();
        if let Some(supplemental) = document.supplemental_markdown.as_deref() {
            content.push_str("\n\n## 運営者による補足\n\n");
            content.push_str(supplemental.trim());
            content.push('\n');
        }
        generated.push(GeneratedLegalDocument {
            kind: document.kind,
            slug: document.slug.clone(),
            version: document.version,
            effective_date: document.effective_date.clone(),
            language: document.language.clone(),
            required: document.required,
            title: document.kind.title_ja().to_string(),
            filename: file.filename.clone(),
            public_path: document.kind.public_path().to_string(),
            content,
            policy_snapshot_revision: snapshot_revision.clone(),
            authoritative_language: document.language.clone(),
            reference_translation: false,
            translation_revision: None,
            translation_of_version: None,
        });
        for translation in &document.translations {
            generated.push(GeneratedLegalDocument {
                kind: document.kind,
                slug: document.slug.clone(),
                version: document.version,
                effective_date: document.effective_date.clone(),
                language: translation.language.clone(),
                required: document.required,
                title: translation.title.clone(),
                filename: format!("{}.{}.md", document.slug, translation.language),
                public_path: document.kind.public_path().to_string(),
                content: translation.body_markdown.clone(),
                policy_snapshot_revision: snapshot_revision.clone(),
                authoritative_language: document.language.clone(),
                reference_translation: true,
                translation_revision: Some(translation.revision),
                translation_of_version: Some(translation.translation_of_version),
            });
        }
    }
    generated
}
