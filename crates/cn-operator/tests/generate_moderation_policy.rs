//! moderation-policy 文書の生成 contract（`generate.rs` から分離。#617 / #1054）。

use kukuri_cn_operator::{generate_all, load_and_validate};

mod generate_support;
use generate_support::{config_with_safety_providers, doc};

#[test]
fn moderation_policy_describes_scan_flow_and_appeals() {
    // #617 T6: moderation-policy が「未提供」ではなく走査の流れ・申し立て導線を説明する。
    let yaml = config_with_safety_providers("      hosting: self_host\n");
    let resolved = load_and_validate(&yaml).unwrap();
    let policy = doc(&generate_all(&resolved), "moderation-policy.md");
    for needle in [
        "走査と判定の流れ",
        "fail-closed",
        "視覚言語モデル",
        "Match Data",
        "申し立て",
        // #1054 / ADR 0028 §8: nsfw / objectionable は content advisory 付きで索引、trust 寄与 0。
        "content advisory",
        "label（content advisory 付きで索引）",
        "寄与 0",
    ] {
        assert!(policy.contains(needle), "missing: {needle}");
    }
    assert!(!policy.contains("未提供"));

    // operator が exclude に厳格化すると、文書もその扱いを表示する。
    let strict = config_with_safety_providers(
        "      hosting: self_host\n  moderation:\n    general_action: exclude\n",
    );
    let resolved = load_and_validate(&strict).unwrap();
    let policy = doc(&generate_all(&resolved), "moderation-policy.md");
    assert!(
        policy.contains("exclude（除外。索引に入れない）"),
        "{policy}"
    );
}
