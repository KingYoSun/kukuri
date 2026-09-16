//! モデレーションポリシー文書の生成（`docs.rs` から分離。#1054）。
//!
//! nsfw / objectionable の扱い（ADR 0028 §8）は operator config の `general_action` を反映して
//! 表示する。文書 version の運用は `docs/runbooks/community-node-operator-docs.md` を参照。

use std::fmt::Write as _;

use crate::capability::Capability;
use crate::config::{LegalDocumentKind, ResolvedConfig};
use crate::docs::{header, planned_section};
use crate::safety_config::GeneralAction;

pub(crate) fn gen_moderation_policy(config: &ResolvedConfig) -> String {
    let mut s = header(
        config,
        "モデレーションポリシー",
        Some(LegalDocumentKind::ModerationPolicy),
    );
    let _ = writeln!(s, "\n## authority scope\n");
    let _ = writeln!(
        s,
        "本ノードの moderation / trust signal は、本ノードの authority scope 内でのみ意味を持ちます。\
         これらは network-wide command ではなく、他ノード・client が任意に採用し得る optional trust input です。\n"
    );
    if config.enabled(Capability::Moderation) || config.enabled(Capability::CommunityLocalTrust) {
        let _ = writeln!(s, "## 走査と判定の流れ\n");
        let _ = writeln!(
            s,
            "- 索引対象は走査後にのみ索引へ入ります（index_before_scan は無効。fail-closed）。"
        );
        let _ = writeln!(
            s,
            "- 既知一致照合（known-match）と分類器（OpenAI 互換の視覚言語モデル）で本文テキスト・\
             メディアを走査します。走査失敗・プロバイダ不達・メディア不達は許可へ落とさず保留します。"
        );
        let _ = writeln!(
            s,
            "- 判定は scan verdict として保存され、非許可・重大への変化は索引から除外されます。"
        );
        let _ = writeln!(
            s,
            "- 判定に基づく moderation event は署名付きで発行され、risk signal は根拠つきで保存されます。"
        );
        let general_action = config
            .raw
            .safety
            .as_ref()
            .and_then(|safety| safety.moderation.general_action)
            .unwrap_or_default();
        let _ = writeln!(
            s,
            "- 性的表現（nsfw）や real-world crimes・hate・harassment・animal abuse（objectionable）の疑いは、\
             重大判定とは別に扱います。本ノードの扱い: {}。既定の `label` では索引から除外せず、\
             利用者向けの content advisory（`adult` / `sensitive`）を付けて索引へ入れます。\
             この advisory は本ノード単独の判定で、投稿の署名済みラベルではありません。",
            match general_action {
                GeneralAction::Label => "label（content advisory 付きで索引）",
                GeneralAction::Hold => "hold（保留。索引に入れない）",
                GeneralAction::Exclude => "exclude（除外。索引に入れない）",
            }
        );
        let _ = writeln!(
            s,
            "- nsfw / objectionable の判定は trust の評価値に寄与しません（寄与 0）。利用者向けの trust 表示には\
             根拠と申し立て状態を確認できる項目として残ります。"
        );
        let _ = writeln!(
            s,
            "- 照合プロバイダの Match Data・モデルの生応答は保存・配布せず、AI の入力にも使いません。\n"
        );
        let _ = writeln!(s, "## 申し立て（異議）\n");
        let _ = writeln!(
            s,
            "本ノードが発行した moderation advisory（risk signal）へは、通報導線から申し立てできます。\
             係争中の寄与は据え置かれ、認容された場合は寄与から除外され、必要に応じて訂正信号を\
             再発行します。\n"
        );
    } else {
        let _ = writeln!(
            s,
            "本ノードでは moderation / trust signal を有効化していません。\n"
        );
    }
    let _ = writeln!(s, "## ログ保持\n");
    let _ = writeln!(
        s,
        "- モデレーションログ保持期間: {} 日\n",
        config.raw.retention.moderation_logs_days
    );
    s.push_str(&planned_section(config));
    s
}
