# 2026-07-30 — #420 非決定論的 moderation（VLM）実装

参照: [Issue #420](https://github.com/KingYoSun/kukuri/issues/420) /
ADR 0028（§7 実装追補を本実装で追加）/ ADR 0027（共通枠組み）/ ADR 0025 §2.3 / ADR 0026 /
`docs/runbooks/openai-compatible-vlm.md`（operator 手順）

## 実装範囲

- **`crates/cn-safety-vlm`（新規）**: OpenAI-compatible VLM provider（`openai-compatible-vlm`）。
  `POST /v1/chat/completions`（chat / vision。media は一時 fetch → data URL）。応答解釈は
  `json`（厳密 JSON 契約）/ `guard`（SingGuard 系: safe|unsafe + `<answer>`、score は先頭
  トークン logprob）の 2 モード。API key は optional（self-host 無認証を許容）、endpoint /
  model は operator 必須指定。`known_hash_match` を設定する経路が構造的に無い
  （basis は常に `ClassifierScore`）。guard カテゴリは critical へ写像しない。
- **閾値（`cn-safety`）**: `unknown_csam_score_threshold` → `suspected_threshold`
  （serde alias で旧名受理、既定 80→70、`policy_version` を `2026-07-public-node-v2` に）。
  general route にも閾値 gating（score/confidence を持つ検知のみ。categorical は従来どおり）。
  critical fail-closed 閾値は共通 1 本（ADR 0028 §6 → §7.1 で決定）。
- **visibility（`cn-safety` + `cn-safety-runtime`）**: `SafetyPolicy.suspected_signal_visibility`
  （既定 Local、operator 可変）。artifact 生成の `visibility_for` が ClassifierScore + content
  category ありのときのみ policy を採用（operational fail-closed は常に Local）。
  `default_visibility_for` / cn-trust `disclosure.rs`（ADR 0026 §6.3 の cross-node pull =
  confirmed のみ）は不変。
- **derived 検索タグ**: `ProviderScanResult.derived_tags` + 純関数 `derived_tags_for_index`
  （`allow` verdict のみ / critical・Match Data・生スコア除外 / 正規化・dedupe）。
  `SafetyScanReport.derived_tags` に適用済みの値を載せ、indexer は text へ相乗り
  （`IndexedEntry` にタグ列は追加しない）。provider 側でも critical 検知時はタグを空にする
  （二重防御）。
- **ingest（`cn-indexer`）**: `has_unscanned_media` の早期 de-index を撤去し、media 参照ごとに
  `with_media_hint` scan を発行。いずれか非 allow なら post 全体を index しない（worst-case
  合成）。VLM / `MediaFetcher` 未構成なら media scan は Unavailable → fail-closed（従来挙動を
  標準経路経由で保存）。
- **appeal / operator レビュー（`cn-core` / `cn-user-api` / `cn-cli`）**:
  `safety_appeals.rs`（遷移ガード `None→Disputed→Cleared|None`、メタデータ編集
  = `operator_review` 有効時のみ、訂正 signal 再発行 + 旧 signal 失効）。申し立ては既存
  `POST /v1/report` の optional `appeal.risk_signal_id`（専用 endpoint / manifest 変更なし）。
  `cn-cli moderation`（list-signals / show / dispute / clear / reject / edit / reissue）。
  署名済み moderation event は不変（是正は risk signal 側）。`Cleared` は配布に残して伝播し、
  `trust_risk_inputs_from` の既存 Cleared 除外で trust 寄与が戻る。
- **結線（`cn-core` / `cn-operator` / `cn-indexer`）**: `resolve_provider` に feature
  `safety-vlm-provider` の arm（general / unknown_csam slot 専用）。operator config
  `safety.moderation`（threshold 1-100 検証 / visibility / operator_review）。readiness に
  `classifier_providers_resolvable`（14 項目に）。env は `.env.community-node.example` 参照。

## ADR 0028 contract（12 本）と配置

| contract | 配置 |
|---|---|
| vlm_provider_is_openai_compatible_and_operator_owned | cn-safety-vlm/tests/provider_contract.rs |
| vlm_basis_is_classifier_score_never_confirmed | 同上 |
| suspected_threshold_default_0_7_operator_tunable | cn-safety/tests/policy_router.rs |
| high_confidence_critical_is_fail_closed_indexing | 同上 |
| advisory_is_network_distributable_per_visibility | cn-safety-runtime/tests/orchestrator.rs |
| false_positive_appeal_path_exists | cn-core/tests/safety_appeals.rs（DB） |
| appeal_cleared_propagates_and_reverts_trust_contribution | 同上 |
| general_moderation_feeds_trust_relative_component | cn-core/tests/trust_inputs.rs |
| critical_suspected_feeds_trust_absolute_component | 同上 |
| derived_tags_only_for_allow_media | cn-safety/tests/derived_tags.rs（indexer 面は ingestion_contracts.rs） |
| derived_tags_exclude_critical_and_match_data | cn-safety/tests/derived_tags.rs |
| operator_review_can_edit_detection_metadata | cn-core/tests/safety_appeals.rs（DB） |

## 実機での確認（2026-07-30）

self-host vLLM（`inclusionAI/SingGuard-2b`、guard モード、無認証）に対して
`cargo test -p kukuri-cn-safety-vlm --test live_endpoint -- --ignored`:

- 無害テキスト → `Completed` / 検知なし → verdict `Allow`（Clean）。
- phishing テキスト → `unsafe` + `<answer>` カテゴリ、logprob 由来 score 100 →
  general route で `Exclude`（critical にはならない）。
- 1x1 PNG（data URL 経由の vision 入力）→ scan 成功 / allow。

## 維持した境界 / 意図的にやらないこと

- `MediaFetcher` の本番実装（iroh-blobs からの blob 一時 fetch）は後続 Issue（fetcher 未構成の
  media scan は fail-closed で、従来と同じく media 参照 post は index されない）。
- タグ語彙の標準化・サムネイル代替表示・operator レビュー UI（#382）・prompt / モデル選定の
  標準化は ADR 0028 §4 のとおり後続。
- cn-trust の cross-node pull（confirmed のみ開示）は不変。suspected advisory の配布
  （`list_distributable_*`）とは別チャネル。
- ingest loop の起動は #406 の保留のまま（結線のみ）。
