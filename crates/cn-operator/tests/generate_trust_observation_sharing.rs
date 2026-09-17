//! 観測提供の任意文書（ADR 0026 §8.5、#1061）の生成・検証 contract（`generate.rs` から分離）。

use kukuri_cn_operator::{
    SAMPLE_CONFIG, generate_legal_documents, load_and_validate, policy_snapshot_revision,
};

/// SAMPLE_CONFIG に観測提供の任意文書（ADR 0026 §8.5、#1061）を加えた config。
fn config_with_trust_observation_sharing(required: bool, slug: &str) -> String {
    SAMPLE_CONFIG.replace(
        "    - kind: rights_infringement\n      slug: rights_infringement\n      version: 1\n      effective_date: 2026-09-02\n      language: ja\n",
        &format!(
            "    - kind: rights_infringement\n      slug: rights_infringement\n      version: 1\n      effective_date: 2026-09-02\n      language: ja\n    - kind: trust_observation_sharing\n      slug: {slug}\n      version: 1\n      effective_date: 2026-09-18\n      language: ja\n      required: {required}\n"
        ),
    )
}

#[test]
fn trust_observation_sharing_document_is_optional_and_published_only_when_configured() {
    // 公開していない node の文書列には現れない（既存 node の snapshot を変えない）。
    let baseline = load_and_validate(SAMPLE_CONFIG).unwrap();
    assert!(
        generate_legal_documents(&baseline)
            .iter()
            .all(|document| document.slug != kukuri_cn_operator::TRUST_OBSERVATION_SHARING_SLUG)
    );

    let yaml = config_with_trust_observation_sharing(false, "trust_observation_sharing");
    let resolved = load_and_validate(&yaml).expect("optional sharing document validates");
    let document = generate_legal_documents(&resolved)
        .into_iter()
        .find(|document| document.slug == kukuri_cn_operator::TRUST_OBSERVATION_SHARING_SLUG)
        .expect("sharing document is generated");
    assert!(!document.required, "観測提供は任意同意");
    assert_eq!(document.filename, "trust-observation-sharing.md");
    for fact in [
        "ブロック・ミュートした相手の公開鍵",
        "提供先はこの community node だけです",
        "180 日",
        "30 日",
        "同意を取り消すと",
    ] {
        assert!(document.content.contains(fact), "missing `{fact}`");
    }
    assert_ne!(
        policy_snapshot_revision(&resolved),
        policy_snapshot_revision(&baseline),
        "任意文書の追加は snapshot に含まれる"
    );
}

#[test]
fn trust_observation_sharing_document_must_be_optional_with_fixed_slug() {
    let required = config_with_trust_observation_sharing(true, "trust_observation_sharing");
    let error = load_and_validate(&required).expect_err("required sharing document must fail");
    assert!(
        error.to_string().contains("required: false"),
        "got: {error}"
    );

    let renamed = config_with_trust_observation_sharing(false, "observation_sharing");
    let error = load_and_validate(&renamed).expect_err("renamed sharing document must fail");
    assert!(
        error.to_string().contains("trust_observation_sharing"),
        "got: {error}"
    );
}
