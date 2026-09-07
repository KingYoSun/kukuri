# Issue #928: capabilityの現行状態と凍結境界の説明同期

- 種別docs、区分A、Scope revision `2026-09-08-872-C-CN-3-v1`。
- before `ac9946d66aaf197204f669a10701fac8a13877e9`。
- REFACTORING凍結境界8はPlanned3件と記載していたが、`cn-operator/src/capability.rs::availability`は全件Availableを返す。`lib.rs`、ADR 0025の2026-08-16改訂、#617の`23a7284e`と不一致だった。

| AC / INVAR | 作業と確認 |
| --- | --- |
| AC-1 | 凍結境界8を現行Available、配備ごとの設定/readinessと区別する説明へ同期。将来向けPlannedの区分と公開契約の凍結は維持 |
| AC-2 | ADR 0027の旧Planned/昇格条件本文を保存し、冒頭へ起草時記録である旨と後継ADR 0025へのリンクを追記。AGENTS/PLANS/docs README/runbooks/.githubに同じ旧3件指示の引用なし |
| AC-3 | `git diff --check`、記載path/commandとリンクを確認、`cargo xtask oversized-files`を実行 |
| INVAR-1 | 差分はREFACTORING/ADR補足/本記録のみ。製品・manifest値・有効化条件・法務/同意/送信規則・既存testは無変更 |

既存`cn-operator/tests/manifest_golden.rs`と`generate.rs`のavailable_enabled/planned_enabled空、disabled表示、`generated_docs_contain_no_planned_wording_after_promotion`を根拠として確認。文書同期で製品testの変更は不要。
独立監査は区分Aの最低条件ではないが、今回ユーザーが全10Issueについて指定したため固定PR headで行う。結果とCI/merge対象はIssue/PRへ記録する。文書のみのためFastのpath triggerに該当しない場合も、未実行を成功とは表記しない。
Rollbackは本Issueの文書PRの単独revert。過去ADR本文の書換え、availabilityの巻戻し、Planned削除は行わない。
