# 2026-09-16 — #1054 CN 側: nsfw / objectionable を content advisory 付きで index する

参照: [Issue #1054](https://github.com/KingYoSun/kukuri/issues/1054)（親 #1051 の C2）/ ADR 0028 §8 /
ADR 0026 §7 / ADR 0025 §7 / ADR 0027 §8 / `docs/runbooks/community-node-production-rollout.md` §5.7 /
`docs/runbooks/community-node-operator-docs.md`「文書 version を上げるときの注意」

## 範囲と判定

- 区分 C。Scope revision 2026-09-15、基準 commit `da781131`（main）。実装計画は 2026-09-16 承認。
- 本記録は実装 PR の作業報告。本番反映と手動確認は merge 後に行い、本記録へ追記する。
- 承認時の判断: moderation-policy 文書の本番 `version: 2` 反映は C4（#1056）の利用規約改訂と同時に行い、
  再同意を 1 回にまとめる。C2 では文言・golden・運用手順まで（AC-8 の範囲）。
- 対象外: client 側の表示・取得ゲート（C3 #1055）、一括照会 API・タイムライン合成・利用規約改訂（C4 #1056）、
  #1050 の残件（#1065）、operator 訂正値の保護（#1058）。

## 修正前の再現

- 現行実装（基準 commit）では `SafetyPolicy::public_node_default().on_high_confidence_nsfw = Exclude` で
  nsfw suspected が index から除外され、`SafetyVerdict` に advisory 欄が無く、`IndexEntryView` に
  ラベル欄が無い。trust は critical 以外を全て相対成分に入れ、nsfw High signal が −0.588 / 件で加算される
  （#1051 本文の本番実レコード）。
- 実装前に赤を確認した contract（コンパイル失敗を含む）: `general_nsfw_is_indexed_with_advisory_label`
  （`advisory_labels` / `Objectionable` / `general_action` が無い）、`general_action_operator_tunable_stricter_only`、
  `objectionable_category_separated_from_nsfw`。実装後に既存の
  `general_moderation_is_separate_route_from_critical`（nsfw → Exclude を前提）が赤になり、ADR 0028 §8.1 の
  advisory 付き allow へ再表現した。

## 実装

| 層 | 変更 |
| --- | --- |
| `crates/cn-safety` | `SafetyCategory::Objectionable`、`is_advisory_only()` / `advisory_display_label()`（adult / sensitive）、`SafetyVerdict.advisory_labels`（`#[serde(default)]`、ラベル付き allow のみ非空）、`is_labeled_allow()`、`ContentAdvisory` / `AdvisorySubjectKind`（wire 要素。ts derive）、`GeneralAction { Label, Hold, Exclude }`、`SafetyPolicy.general_action`（旧 `on_high_confidence_nsfw` は alias で hold / exclude のみ受理）、`route()` 規則 8（nsfw / objectionable は `general_action`、同一 result に複数の一般ラベルがあれば非 index 側を優先）、`policy_version` v3 |
| `crates/cn-safety-vlm` | guard 写像 A → Nsfw、B / C / G → Objectionable、json `objectionable` 受理 |
| `crates/cn-cli` / `cn-user-api` 運営画面 | category 引数・select に `objectionable` |
| `crates/cn-safety-runtime` | `build_artifacts`: ラベル付き allow で `RiskLabel` event + `Low` signal（basis ClassifierScore、visibility は `suspected_signal_visibility`）。`scan_and_record_inner` は signal を verdict より先に永続化し、`content_advisories_for` で signal id 込みの advisory を verdict 行へ保存。`StoredVerdictRecord.advisories` / `VerdictPersistMeta.advisories`、再利用時は自 subject 分だけ復元。`SafetyArtifactStore::persist_advisories`、`SafetyScanService::persist_advisories`。`SafetyRuntimeConfig.general_action` → `resolve_safety_policy` |
| `crates/cn-core` | migration `202609160001_scan_verdict_advisory_labels.sql`（`advisory_labels JSONB NOT NULL DEFAULT '[]'`、`IF NOT EXISTS` で冪等）、`StoredScanVerdict.advisory_labels` / `own_advisories()` / `to_record()` の復元、`update_scan_verdict_advisories`（値が同じなら no-op）、`SurfaceableEntry`（`filter_surfaceable` が最新 verdict 由来の advisory を同伴）、`PgSafetyArtifactStore::persist_advisories` |
| `crates/cn-trust` | `signal_contribution` は advisory-only category で常に 0（`component` は `Relative` のまま basis に残る） |
| `crates/cn-indexer` | text + 各 blob の advisory の和集合を post verdict 行へ確定、`IndexedEntry.content_advisories`（投影には書かず gate が充填）、env `COMMUNITY_NODE_SAFETY_GENERAL_ACTION`（label / hold / exclude、`allow` と未知値は起動失敗。`validate-config` も同経路） |
| `crates/cn-protocol` / `cn-user-api` / TS | `IndexEntryView.content_advisories`（`#[serde(default)]`、常に配列で出力）、`index_query_response` で充填、`ContentAdvisory` / `AdvisorySubjectKind` / `SafetyCategory.objectionable` を `types.generated.ts` へ再生成 |
| `crates/cn-operator` | `safety.moderation.general_action`（`skip_serializing_if = None` で未指定時は法務 snapshot 不変）、`on_high_confidence` deprecated（`ResolvedConfig::warnings` を `validate-config` 等が表示）、tfvars `safety_general_action`、`gen_moderation_policy` に nsfw / objectionable の扱い（本ノードの設定値を表示）と trust 寄与 0 を追記、サンプル config の deprecated key を除去 |
| infra / compose | terraform module（`variables.tf` validation 付き / `main.tf` 2 箇所 / `env.tftpl`）、`envs/low-cost`、compose の cn-indexer service に `COMMUNITY_NODE_SAFETY_GENERAL_ACTION` |
| docs | rollout runbook §5.7、operator-docs runbook「文書 version を上げるときの注意」、本記録 |

### 設計上の決定（計画の仮定 1〜5 の実装結果）

- `labels` は検知内容のまま。`advisory_labels` は `general_action = label` で `Allow` に落ちた nsfw / objectionable の
  写し。非 index の verdict では常に空。署名済み event の `labels` にも advisory の表示語彙は入れない。
- post の `content_advisories` は post の verdict 行 `advisory_labels` に text + blob の和集合として保持
  （要素は `subject_kind = post_id | blob_cid`）。query 境界は既存の verdict FK join だけで導出し、
  index entry に列を増やさない。indexer は media scan 確定後に `persist_advisories` で和集合を確定させ、
  値が同じなら DB には書かない（`updated_at` も動かない）。
- `SafetyPolicy` の serde 表現（`general_action` への改名 + v3）で scan 構成 fingerprint が変わり、
  反映後 1 巡だけ全件再 scan が起きる。これが ADR 0028 §8.9 の backfill で、専用 migration は無い。
- 旧 `on_high_confidence_nsfw` は alias で `hold` / `exclude` だけ受理し、`allow` / `quarantine` は
  deserialize エラー。runtime は operator config を直接読まないため、実運用の影響は env の値域検証のみ。
- operator config の `safety.policy_version`（表示・snapshot 用）は触っていない。router の v3 とは別物。

## 契約と証跡の対応

| AC / INVAR | test（contract 名） | 配置 |
| --- | --- | --- |
| AC-1 | `general_nsfw_is_indexed_with_advisory_label` | cn-safety `policy_router.rs` / cn-indexer `ingestion_contracts.rs` |
| AC-2 | `objectionable_category_separated_from_nsfw` | cn-safety `policy_router.rs` / cn-safety-vlm `provider_contract.rs` |
| AC-3 | `labeled_allow_emits_risk_label_event_and_signal`、`content_advisories_carry_signal_id_and_display_label`、`reused_labeled_allow_restores_advisories_without_artifacts`（TR-10） | cn-safety-runtime `orchestrator.rs` / `reuse.rs` |
| AC-4 | `labeled_allow_text_does_not_short_circuit_media_scan` | cn-indexer `ingestion_contracts.rs` |
| AC-5 | `advisory_labels_migration_defaults_to_empty_and_is_idempotent`、`index_entry_advisories_derive_from_latest_verdict`、`content_advisories_are_separate_from_signed_content_labels`、`index_query_response_wire_shape_is_stable`（`content_advisories: []`） | cn-core `scan_verdict_advisory_migration.rs` / `index_entries.rs`、cn-user-api `index_query.rs`、cn-protocol `index_contract.rs` |
| AC-6 | `general_advisory_contributes_zero_to_trust`（cn-trust / cn-core）、`trust_read_lists_advisory_only_basis_with_zero_contribution`（read + pull 除外） | cn-trust `trust_scoring.rs`、cn-core `trust_inputs.rs`、cn-user-api `trust_relation.rs` |
| AC-7 | `general_action_operator_tunable_stricter_only`（cn-safety / cn-operator）、`general_action_env_rejects_allow_and_unknown`（cn-indexer config）、`safety_moderation_env_wiring_reaches_compose_and_terraform`、tfvars の `safety_general_action` | 各 crate |
| AC-8 | `moderation_policy_describes_scan_flow_and_appeals`（content advisory / 寄与 0 / exclude 表示）、runbook の version 2 手順 | cn-operator `generate.rs`、`docs/runbooks/community-node-operator-docs.md` |
| INVAR-1 | migration test の CHECK / FK 不変、`index_entry_advisories_derive_from_latest_verdict` の exclude で entry 消失、`is_indexable()` 単一判定点（`verdict_is_indexable_only_when_allow`） | cn-core / cn-safety |
| INVAR-2 | 既存 fail-closed test 全件不変（`high_confidence_critical_is_fail_closed_indexing`、`scan_failure_fails_closed_not_allow`、`provider_unavailable_fails_closed`、`index_excludes_unscanned_and_scan_failed` ほか）、`general_action_operator_tunable_stricter_only` の critical 不変 | cn-safety / cn-safety-runtime / cn-indexer |
| INVAR-3 | `content_advisories_are_separate_from_signed_content_labels`、`labeled_allow_emits_risk_label_event_and_signal`（event.labels = 検知ラベル）、basis は常に `ClassifierScore`（`vlm_basis_is_classifier_score_never_confirmed` 維持） | cn-user-api / cn-safety-runtime / cn-safety-vlm |
| INVAR-4 | `trust_read_lists_advisory_only_basis_with_zero_contribution` の pull 除外、`cross_node_pull_discloses_only_confirmed_absolute_component` 維持 | cn-user-api |
| INVAR-5 | 既存 `assert_no_durable_blob_io` / `two_node_remote_media_fetch_is_ephemeral` 維持 | cn-indexer |

### 既存 test の再表現

- `general_moderation_feeds_trust_relative_component` → `spam_malware_phishing_feed_trust_relative_component`
  （ADR 0026 §7.4 の superseded）。cn-trust の相対成分 fixture（`nsfw_input`）は `spam_input` へ、
  cn-e2e `trust_relation_graph.rs` の相対 signal も spam へ。
- `derived_tags_only_for_allow_media` の非 allow 側を spam に変え、ラベル付き allow の media もタグ化する
  ケース（ADR 0028 §8.8）を追加。`flagged_media_post_is_not_indexed_and_tags_do_not_leak` も spam へ。
- `general_moderation_uses_classifier_basis_and_local_visibility`（非 index の一般判定）は spam で固定。
- cn-user-api `trust_read_returns_components_with_basis_and_ignores_reports` の相対成分 signal も spam へ。
- cn-safety-runtime `reuse.rs` の再利用・集約契約は `general_action = Exclude` の policy で固定し、既定の
  label は `reused_labeled_allow_restores_advisories_without_artifacts` が担う。

## inventory / transition

Issue の INV-1〜9 / TR-1〜9 に加え、計画で逆引き追加した INV-10（再利用経路）/ TR-10、INV-11（category
文字列の消費者: cn-cli 引数・運営画面 select・TS union）、INV-12（operator config → 法務 snapshot）を上表の
test に対応付けた。未分類 0。

## 検証

- `cargo test -p kukuri-cn-safety` / `-p kukuri-cn-safety-vlm` / `-p kukuri-cn-safety-runtime` /
  `-p kukuri-cn-trust` / `-p kukuri-cn-indexer` / `-p kukuri-cn-protocol` / `-p kukuri-cn-operator`: 成功。
- `cargo test -p kukuri-cn-core`（DB 非依存分）: 成功。DB 依存分は `cn-test` に記録。
- `cargo check --tests` for cn-user-api / cn-e2e / harness / desktop-runtime: 成功。
- `cargo xtask cn-check`: 成功。
- `cargo xtask ipc-types`: `types.generated.ts` を再生成（`ContentAdvisory` / `AdvisorySubjectKind` /
  `IndexEntryView.content_advisories` / `SafetyCategory` に `objectionable`）。desktop `tsc --noEmit` エラー 0、
  touched ファイルの eslint 成功、`communityIndexPostCardView.test.ts` 8 件成功。
- `terraform fmt -check`（module / envs/low-cost）: 成功。
- `cargo xtask check`（fmt / workspace clippy / tauri check / desktop lint / typecheck）: 成功。
- `cargo xtask test`（workspace 全体）は CI（`Kukuri Fast` の linux-rust-tests ほか）で実行し、PR の check 結果を根拠にする。
- `cargo xtask cn-test` / `cargo xtask cn-e2e`: 下記に記録。

### cn-test

- `cargo xtask cn-test`（cn-postgres / cn-valkey を compose で起動、`KUKURI_CN_RUN_INTEGRATION_TESTS=1`）:
  615 passed / 0 failed / 0 ignored（2026-09-16）。
- 途中で赤になり修正した 2 件（いずれも既定 policy が nsfw = label になったことに伴う既存 test の前提ずれ）:
  - `postgres_store_reuses_verdict_and_does_not_duplicate_artifacts`（cn-core）: ラベル付き allow の再利用で
    2 人目の著者が関連付かなかった（Existing-gap。AC-3 / TR-10 の範囲）。`scan_and_record_inner` の再利用分岐を
    「risk signal を持つ verdict（非 allow またはラベル付き allow）」で著者関連付けするよう修正し、
    memory store 側の `reused_labeled_allow_restores_advisories_without_artifacts` にも 2 人目の著者の検証を追加。
  - `trust_read_returns_components_with_basis_and_ignores_reports`（cn-user-api）: nsfw を相対成分の代表として
    `relative < 0` を期待していたため、spam に切り替えた（nsfw の 0 寄与は
    `trust_read_lists_advisory_only_basis_with_zero_contribution` が固定）。
- 新規 Postgres test: `advisory_labels_migration_defaults_to_empty_and_is_idempotent`、
  `index_entry_advisories_derive_from_latest_verdict`、`content_advisories_are_separate_from_signed_content_labels`、
  `trust_read_lists_advisory_only_basis_with_zero_contribution`。
- 運用注意: `cn-test` と `cn-e2e` は同じ compose stack を `down -v` するため同時実行できない（同時に走らせて
  一度失敗した）。

### cn-e2e

- `cargo xtask cn-e2e`（cn-postgres / cn-valkey / cn-arcadedb を compose で起動）: 全 suite 成功
  （allowed_path 1、denied_paths 1、failure_paths 6、recovery 1、trust_relation_graph 1。2026-09-16）。
  `trust_relation_graph.rs` の相対成分 signal は spam へ切り替えた（advisory-only の nsfw では相対成分が動かない）。

## 独立監査

（PR head 固定後に別コンテキストで実施し追記）

## 本番反映

（merge 後に runbook §1〜§5、§5.7 に従って実施し追記）

## 意図的にやらなかったこと

- 本番 operator-config の `moderation_policy` `version: 2` 反映（C4 と同時に 1 回の再同意へまとめる）。
- client 側の表示・取得ゲート・一括照会 API・利用規約改訂（C3 / C4）。
- `on_high_confidence` の canonical snapshot からの除去（既存 config の snapshot を変えないため受理・警告のみ）。
