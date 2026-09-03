use kukuri_cn_operator::{NodeRole, ProviderHosting, SAMPLE_CONFIG, policy_snapshot_revision};

fn baseline() -> kukuri_cn_operator::ResolvedConfig {
    kukuri_cn_operator::load_and_validate(SAMPLE_CONFIG).expect("sample config must validate")
}

fn revision(config: &kukuri_cn_operator::ResolvedConfig) -> String {
    policy_snapshot_revision(config).expect("legal config has a snapshot")
}

macro_rules! assert_legal_change {
    ($baseline:ident, $baseline_revision:ident, $label:expr, |$config:ident| $body:block) => {{
        let mut changed = $baseline.clone();
        let $config = &mut changed;
        $body
        assert_ne!(
            revision(&changed),
            $baseline_revision,
            "legal input must change the snapshot: {}",
            $label
        );
    }};
}

#[test]
fn every_canonical_legal_input_category_changes_the_snapshot() {
    let baseline = baseline();
    let baseline_revision = revision(&baseline);

    assert_legal_change!(baseline, baseline_revision, "domain", |config| {
        config.raw.server.domain = "other.example".into();
    });
    assert_legal_change!(baseline, baseline_revision, "operator name", |config| {
        config.raw.server.operator_name = "Other Operator".into();
    });
    assert_legal_change!(baseline, baseline_revision, "country", |config| {
        config.raw.server.country = "GB".into();
    });
    assert_legal_change!(baseline, baseline_revision, "contact", |config| {
        config.raw.server.contact = Some("privacy@other.example".into());
    });
    assert_legal_change!(baseline, baseline_revision, "cloud provider", |config| {
        config.raw.server.cloud_provider = Some("ExampleCloud".into());
    });
    assert_legal_change!(baseline, baseline_revision, "region", |config| {
        config.raw.server.region = Some("eu-west-1".into());
    });
    assert_legal_change!(
        baseline,
        baseline_revision,
        "identity disclosure route",
        |config| {
            config
                .raw
                .legal
                .as_mut()
                .unwrap()
                .identity_disclosure_request = "Use the verified operator contact route.".into();
        }
    );

    let capability_changed = SAMPLE_CONFIG.replace("  analytics: false", "  analytics: true");
    let capability_changed = kukuri_cn_operator::load_and_validate(&capability_changed).unwrap();
    assert_ne!(
        revision(&capability_changed),
        baseline_revision,
        "enabled typed capability descriptors are canonical input"
    );

    macro_rules! retention_change {
        ($field:ident) => {
            assert_legal_change!(baseline, baseline_revision, stringify!($field), |config| {
                config.raw.retention.$field += 1;
            });
        };
    }
    retention_change!(connection_logs_days);
    retention_change!(moderation_logs_days);
    retention_change!(report_days);
    retention_change!(report_contact_days);
    retention_change!(tester_feedback_days);
    retention_change!(rights_request_active_days);
    retention_change!(rights_request_resolved_days);
    retention_change!(rights_request_rejected_days);
    retention_change!(rights_request_contact_days);
    retention_change!(rights_request_identity_days);
    retention_change!(rights_request_evidence_days);
    retention_change!(rights_request_history_days);
    retention_change!(operator_audit_days);
    retention_change!(moderation_event_days);
    retention_change!(risk_signal_days);

    assert_legal_change!(baseline, baseline_revision, "safety policy", |config| {
        config.raw.safety.as_mut().unwrap().policy_version = "next-policy".into();
    });
    assert_legal_change!(baseline, baseline_revision, "safety provider", |config| {
        config
            .raw
            .safety
            .as_mut()
            .unwrap()
            .providers
            .known_csam
            .as_mut()
            .unwrap()
            .provider = "other-provider".into();
    });
    assert_legal_change!(baseline, baseline_revision, "provider hosting", |config| {
        config
            .raw
            .safety
            .as_mut()
            .unwrap()
            .providers
            .general
            .as_mut()
            .unwrap()
            .hosting = Some(ProviderHosting::SelfHost);
    });
    assert_legal_change!(baseline, baseline_revision, "authority scope", |config| {
        config
            .raw
            .manifest
            .authority_scope
            .additional_applies_to
            .push("operator_defined_service".into());
    });
    assert_legal_change!(baseline, baseline_revision, "manifest version", |config| {
        config.raw.manifest.manifest_version = "v2".into();
    });
    assert_legal_change!(baseline, baseline_revision, "response target", |config| {
        config
            .raw
            .manifest
            .rights_request_initial_response_target_days += 1;
    });
    assert_legal_change!(baseline, baseline_revision, "document slug", |config| {
        config.raw.legal.as_mut().unwrap().documents[0].slug = "terms_next".into();
    });
    assert_legal_change!(baseline, baseline_revision, "document version", |config| {
        config.raw.legal.as_mut().unwrap().documents[0].version += 1;
    });
    assert_legal_change!(baseline, baseline_revision, "effective date", |config| {
        config.raw.legal.as_mut().unwrap().documents[0].effective_date = "2026-09-03".into();
    });
    assert_legal_change!(
        baseline,
        baseline_revision,
        "authoritative language",
        |config| {
            config.raw.legal.as_mut().unwrap().documents[0].language = "en".into();
        }
    );
    assert_legal_change!(baseline, baseline_revision, "required status", |config| {
        config.raw.legal.as_mut().unwrap().documents[0].required = false;
    });
    assert_legal_change!(
        baseline,
        baseline_revision,
        "operator supplement",
        |config| {
            config.raw.legal.as_mut().unwrap().documents[0].supplemental_markdown =
                Some("Operator-specific term.".into());
        }
    );
}

#[test]
fn technical_identity_secrets_order_and_reference_translation_do_not_change_snapshot() {
    let baseline = baseline();
    let baseline_revision = revision(&baseline);

    let mut technical = baseline.clone();
    technical.raw.server.node_name = Some("Display-only node name".into());
    technical.raw.server.node_id = Some("f".repeat(64));
    technical.raw.manifest.node_role = Some(NodeRole::DefaultOnboardingNode);
    technical.raw.acknowledge_planned_capabilities = false;
    let safety = technical.raw.safety.as_mut().unwrap();
    safety.events.signing_key_secret_id = Some("rotated-signing-key-id".into());
    safety
        .providers
        .known_csam
        .as_mut()
        .unwrap()
        .credential_secret_id = Some("rotated-provider-key-id".into());
    assert_eq!(revision(&technical), baseline_revision);

    let mut reordered = baseline.clone();
    reordered.raw.legal.as_mut().unwrap().documents.reverse();
    assert_eq!(
        revision(&reordered),
        baseline_revision,
        "document display order is not legal identity"
    );

    let with_translation = SAMPLE_CONFIG.replace(
        "      required: true\n    - kind: privacy",
        "      required: true\n      translations:\n        - language: en\n          revision: 99\n          translation_of_version: 1\n          title: Revised reference\n          body_markdown: Revised reference-only text.\n    - kind: privacy",
    );
    let with_translation = kukuri_cn_operator::load_and_validate(&with_translation).unwrap();
    assert_eq!(revision(&with_translation), baseline_revision);
}
