//! trust scoring の contract テスト（ADR 0026 §2.3 / §6.2, Issue #415）。DB 不要。
//!
//! テスト名は ADR 0026 の必須 contract 名に対応する。

use chrono::{DateTime, Duration, Utc};

use kukuri_cn_safety::{AppealStatus, Basis, SafetyCategory, Severity, Visibility};
use kukuri_cn_trust::{
    TrustComponentKind, TrustParams, TrustRiskInput, TrustRiskInputs, UniformRelationWeight,
    build_trust_read, compose_trust, trust_component_for,
};

#[allow(clippy::unwrap_used)] // test fixture helper
fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-07-02T09:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn input(
    id: &str,
    component: TrustComponentKind,
    category: SafetyCategory,
    severity: Severity,
    confidence: Option<u8>,
    persisted_at: DateTime<Utc>,
) -> TrustRiskInput {
    TrustRiskInput {
        signal_id: id.to_string(),
        issuer_node_id: "issuer-node".to_string(),
        component,
        category,
        severity,
        basis: Basis::KnownHashMatch,
        confidence,
        visibility: Visibility::Local,
        appeal_status: AppealStatus::None,
        expires_at: None,
        persisted_at,
    }
}

fn csam_input(id: &str, persisted_at: DateTime<Utc>) -> TrustRiskInput {
    input(
        id,
        TrustComponentKind::Absolute,
        SafetyCategory::Csam,
        Severity::Critical,
        Some(100),
        persisted_at,
    )
}

fn nsfw_input(id: &str, persisted_at: DateTime<Utc>) -> TrustRiskInput {
    input(
        id,
        TrustComponentKind::Relative,
        SafetyCategory::Nsfw,
        Severity::High,
        Some(100),
        persisted_at,
    )
}

// --- 合成式（§6.2） ---

#[test]
fn trust_is_clamped_to_unit_interval() {
    let params = TrustParams::default();
    // absolute=-1, relative=-1 → 生値 (2*-1 + -1)/2 = -1.5 だが最終値は -1 で止まる。
    let composed = compose_trust(&params, -1.0, -1.0);
    assert_eq!(composed.trust, -1.0);
    // 成分に範囲外の値が渡っても ±1 に丸めてから合成する。
    let composed = compose_trust(&params, -5.0, 3.0);
    assert!((-1.0..=1.0).contains(&composed.trust));
}

#[test]
fn trust_absolute_negative_is_weighted_double() {
    let params = TrustParams::default();
    // 絶対成分がマイナスのとき重み 2 倍（distrust 支配）。相対が最良でも救済されない。
    let negative = compose_trust(&params, -1.0, 1.0);
    assert_eq!(negative.w_abs_applied, 2.0);
    // §6.2 の式そのまま: (w_abs * absolute + relative) / 2 = (2.0 * (-1.0) + 1.0) / 2 = -0.5
    assert_eq!(negative.trust, -0.5);
    assert!(negative.trust < 0.0, "良好な相対指標で薄まってはいけない");
    // プラスのときは 1 倍。
    let positive = compose_trust(&params, 0.5, 0.0);
    assert_eq!(positive.w_abs_applied, 1.0);
    assert_eq!(positive.trust, 0.25);
}

#[test]
fn trust_composition_weights_are_operator_tunable() {
    // operator が w_abs を変えると合成結果が変わる。
    let default_params = TrustParams::default();
    let tuned = TrustParams {
        w_abs_negative: 4.0,
        ..TrustParams::default()
    };
    let a = compose_trust(&default_params, -0.4, 0.2);
    let b = compose_trust(&tuned, -0.4, 0.2);
    assert_ne!(a.trust, b.trust);
    assert_eq!(b.w_abs_applied, 4.0);

    // env 相当の lookup から可変（未設定キーは既定値、値は上書き）。
    let params = TrustParams::from_lookup(|name| {
        (name == "COMMUNITY_NODE_TRUST_W_ABS_NEGATIVE").then(|| "3.5".to_string())
    })
    .unwrap();
    assert_eq!(params.w_abs_negative, 3.5);
    assert_eq!(params.w_abs_positive, 1.0);
    assert_eq!(params.relative_half_life_days, 30.0);

    // 不正値は黙って既定値に倒さず Err（operator の設定ミスを検出する）。
    assert!(
        TrustParams::from_lookup(|name| {
            (name == "COMMUNITY_NODE_TRUST_W_ABS_NEGATIVE").then(|| "not-a-number".to_string())
        })
        .is_err()
    );
    assert!(
        TrustParams::from_lookup(|name| {
            (name == "COMMUNITY_NODE_TRUST_RELATIVE_HALF_LIFE_DAYS").then(|| "0".to_string())
        })
        .is_err()
    );
}

// --- 単一絶対スカラーにしない / 成分分離（§2.3） ---

#[test]
fn trust_is_not_single_absolute_scalar() {
    let inputs = TrustRiskInputs {
        absolute: vec![csam_input("sig-abs", now())],
        relative: vec![nsfw_input("sig-rel", now())],
    };
    let view = build_trust_read(
        "pubkey-1",
        &inputs,
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    // read は絶対 / 相対成分を分離して返し、合成値だけの単一スカラーにしない。
    assert_ne!(view.absolute, view.relative);
    assert!(view.absolute < 0.0);
    assert!(view.relative < 0.0);
    // 断定ラベルは存在しない（連続値 + 根拠のみ）。
    assert!(!view.basis.is_empty());
}

#[test]
fn trust_separates_absolute_and_relative_indicators() {
    // CSAM（critical）は絶対成分のみ、nsfw は相対成分のみに効く。
    let absolute_only = TrustRiskInputs {
        absolute: vec![csam_input("sig-abs", now())],
        relative: vec![],
    };
    let view = build_trust_read(
        "pubkey-1",
        &absolute_only,
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    assert!(view.absolute < 0.0);
    assert_eq!(view.relative, 0.0);

    let relative_only = TrustRiskInputs {
        absolute: vec![],
        relative: vec![nsfw_input("sig-rel", now())],
    };
    let view = build_trust_read(
        "pubkey-1",
        &relative_only,
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    assert_eq!(view.absolute, 0.0);
    assert!(view.relative < 0.0);
}

#[test]
fn trust_reflects_risk_signals_split_by_category() {
    // category → 成分の振り分け規則（#406 の供給契約と同じ規則を scoring 側でも公開する）。
    for category in [
        SafetyCategory::Csam,
        SafetyCategory::Cse,
        SafetyCategory::Grooming,
    ] {
        assert_eq!(trust_component_for(category), TrustComponentKind::Absolute);
    }
    for category in [
        SafetyCategory::Nsfw,
        SafetyCategory::Spam,
        SafetyCategory::Malware,
        SafetyCategory::Phishing,
    ] {
        assert_eq!(trust_component_for(category), TrustComponentKind::Relative);
    }
}

// --- relation 重み付け（§2.3: 絶対は非依存、相対は依存） ---

#[test]
fn trust_absolute_indicators_not_relation_weighted() {
    let inputs = TrustRiskInputs {
        absolute: vec![csam_input("sig-abs", now())],
        relative: vec![],
    };
    let params = TrustParams::default();
    // relation 重みを 0 にしても絶対成分は 1 ミリも動かない（report-bomb 不動・relation 非依存）。
    let unweighted = build_trust_read("pk", &inputs, now(), &params, &UniformRelationWeight(0.0));
    let weighted = build_trust_read("pk", &inputs, now(), &params, &UniformRelationWeight(1.0));
    assert_eq!(unweighted.absolute, weighted.absolute);
    assert_eq!(unweighted.trust, weighted.trust);
    assert_eq!(unweighted.basis[0].relation_weight, 1.0);
}

#[test]
fn trust_relative_indicators_are_relation_weighted() {
    let inputs = TrustRiskInputs {
        absolute: vec![],
        relative: vec![nsfw_input("sig-rel", now())],
    };
    let params = TrustParams::default();
    let full = build_trust_read("pk", &inputs, now(), &params, &UniformRelationWeight(1.0));
    let half = build_trust_read("pk", &inputs, now(), &params, &UniformRelationWeight(0.5));
    let none = build_trust_read("pk", &inputs, now(), &params, &UniformRelationWeight(0.0));
    // relation 重みが相対成分の実効寄与を変える（viewer / cluster 相対）。
    assert!(full.relative < half.relative);
    assert_eq!(none.relative, 0.0);
    assert_eq!(half.basis[0].relation_weight, 0.5);
}

#[test]
fn trust_resists_mass_report_bombing() {
    // 特定 cluster からの大量シグナルは raw count として効かず、relation 重みで相対化される。
    // （通報そのもの（cn_admin.reports）は reporter identity を持たず trust の入力に一切
    //  ならない — raw count が構造的に入り得ないことが第一の防壁。本テストは第二の防壁 =
    //  相対成分の relation 重み付けを固定する。）
    let bombed: Vec<TrustRiskInput> = (0..100)
        .map(|i| nsfw_input(&format!("bomb-{i}"), now()))
        .collect();
    let inputs = TrustRiskInputs {
        absolute: vec![],
        relative: bombed,
    };
    let params = TrustParams::default();

    // 遠い cluster（重み 0.001）からの 100 件は、近い cluster の 1 件より小さい寄与に留まる。
    let bombed_far = build_trust_read("pk", &inputs, now(), &params, &UniformRelationWeight(0.001));
    let single_near = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![],
            relative: vec![nsfw_input("single", now())],
        },
        now(),
        &params,
        &UniformRelationWeight(1.0),
    );
    assert!(
        bombed_far.relative > single_near.relative,
        "100 件 × 重み 0.001 ({}) は 1 件 × 重み 1.0 ({}) より寄与が小さい（マイナス幅が浅い）こと",
        bombed_far.relative,
        single_near.relative
    );

    // そして絶対成分は件数に関わらず不動。
    assert_eq!(bombed_far.absolute, 0.0);
}

// --- decay（§6.2: 絶対は減衰しない / 相対は半減期） ---

#[test]
fn trust_absolute_component_does_not_decay() {
    let old = now() - Duration::days(365);
    let fresh_view = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![csam_input("sig", now())],
            relative: vec![],
        },
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    let old_view = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![csam_input("sig", old)],
            relative: vec![],
        },
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    // known-hash 由来の絶対成分は 1 年経っても同じ寄与（時間で薄めない）。
    assert_eq!(fresh_view.absolute, old_view.absolute);
    assert_eq!(old_view.basis[0].decay_factor, 1.0);
}

#[test]
fn trust_relative_component_decays_over_time() {
    let params = TrustParams::default(); // 半減期 30 日
    let fresh = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![],
            relative: vec![nsfw_input("sig", now())],
        },
        now(),
        &params,
        &UniformRelationWeight::default(),
    );
    let aged = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![],
            relative: vec![nsfw_input("sig", now() - Duration::days(30))],
        },
        now(),
        &params,
        &UniformRelationWeight::default(),
    );
    // 1 半減期でちょうど寄与が半分になる。
    let ratio = aged.relative / fresh.relative;
    assert!(
        (ratio - 0.5).abs() < 1e-9,
        "30 日（1 半減期）で寄与が半分になること: ratio={ratio}"
    );
    // 半減期は operator 可変: 短くすればより速く減衰する。
    let short = TrustParams {
        relative_half_life_days: 7.0,
        ..TrustParams::default()
    };
    let aged_short = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![],
            relative: vec![nsfw_input("sig", now() - Duration::days(30))],
        },
        now(),
        &short,
        &UniformRelationWeight::default(),
    );
    assert!(
        aged_short.relative > aged.relative,
        "短半減期はより浅い寄与（減衰が速い）"
    );
}

// --- appeal 反映（§6.2） ---

#[test]
fn trust_appeal_pending_holds_contribution() {
    // pending（Disputed）は寄与据え置き: 申し立て中に勝手に緩めない。
    let mut disputed = csam_input("sig-disputed", now());
    disputed.appeal_status = AppealStatus::Disputed;
    let view = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![disputed],
            relative: vec![],
        },
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    assert!(view.absolute < 0.0, "pending は寄与したまま");
    assert_eq!(view.basis[0].appeal_status, AppealStatus::Disputed);
}

#[test]
fn trust_appeal_accepted_excludes_contribution() {
    // accepted（Cleared）は供給層で除外されるが、万一 scoring まで届いても寄与させない
    // （defense in depth）。
    let mut cleared = csam_input("sig-cleared", now());
    cleared.appeal_status = AppealStatus::Cleared;
    let view = build_trust_read(
        "pk",
        &TrustRiskInputs {
            absolute: vec![cleared],
            relative: vec![],
        },
        now(),
        &TrustParams::default(),
        &UniformRelationWeight::default(),
    );
    assert_eq!(view.absolute, 0.0);
    assert!(view.basis.is_empty(), "除外された signal は根拠にも出ない");
}

// --- 根拠つき read（trust-semantics §4） ---

#[test]
fn trust_read_is_explainable_with_basis() {
    let inputs = TrustRiskInputs {
        absolute: vec![csam_input("sig-abs", now())],
        relative: vec![nsfw_input("sig-rel", now() - Duration::days(30))],
    };
    let view = build_trust_read(
        "pubkey-1",
        &inputs,
        now(),
        &TrustParams::default(),
        &UniformRelationWeight(0.8),
    );
    assert_eq!(view.target_id, "pubkey-1");
    assert_eq!(view.basis.len(), 2);

    let abs = view
        .basis
        .iter()
        .find(|e| e.signal_id == "sig-abs")
        .unwrap();
    // issuer / basis / confidence / visibility / expiry / appeal を説明できる。
    assert_eq!(abs.issuer_node_id, "issuer-node");
    assert_eq!(abs.component, TrustComponentKind::Absolute);
    assert_eq!(abs.category, SafetyCategory::Csam);
    assert_eq!(abs.basis, Basis::KnownHashMatch);
    assert_eq!(abs.confidence, Some(100));
    assert_eq!(abs.visibility, Visibility::Local);
    assert_eq!(abs.appeal_status, AppealStatus::None);
    assert_eq!(abs.decay_factor, 1.0);
    assert_eq!(abs.relation_weight, 1.0);

    let rel = view
        .basis
        .iter()
        .find(|e| e.signal_id == "sig-rel")
        .unwrap();
    // 実効寄与の内訳（decay / relation 重み）まで説明できる。
    assert!((rel.decay_factor - 0.5).abs() < 1e-9);
    assert_eq!(rel.relation_weight, 0.8);
    assert!((rel.contribution - rel.raw_contribution * rel.decay_factor * 0.8).abs() < 1e-12);

    // 合成値と成分・適用重みが view から再構成できる（説明可能性）。
    let recomposed = compose_trust(&TrustParams::default(), view.absolute, view.relative);
    assert_eq!(view.trust, recomposed.trust);
    assert_eq!(view.w_abs_applied, recomposed.w_abs_applied);
}

// --- scenario: CSAM 系 risk は relation / 通報数で揺れない ---

#[test]
fn csam_absolute_component_is_immune_to_relation_and_mass_signals() {
    let params = TrustParams::default();
    let base = TrustRiskInputs {
        absolute: vec![csam_input("sig-csam", now() - Duration::days(180))],
        relative: vec![],
    };
    let view_alone = build_trust_read("pk", &base, now(), &params, &UniformRelationWeight(0.0));

    // 大量の相対シグナルを足しても、relation 重みを変えても、絶対成分は同じ。
    let mut with_noise = base.clone();
    with_noise.relative = (0..50)
        .map(|i| nsfw_input(&format!("noise-{i}"), now()))
        .collect();
    let view_noisy = build_trust_read(
        "pk",
        &with_noise,
        now(),
        &params,
        &UniformRelationWeight(1.0),
    );

    assert_eq!(view_alone.absolute, view_noisy.absolute);
    assert!(view_alone.absolute <= -1.0 + 1e-12);
    // 絶対成分マイナスなので合成でも distrust が支配的（相対が最悪でも -1 未満に落ちない）。
    assert!(view_noisy.trust <= -0.5);
    assert!(view_noisy.trust >= -1.0);
}
