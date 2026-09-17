# #1061 trust 絶対値と閲覧者別 relation 値の合算・ブロック/ミュート観測の作業記録

## 現在の状態

- リスク区分 C、Scope revision: 2026-09-15-r2、基準 commit: `d5e6806a`（PR1 の分岐元）。
- 2026-09-17 にユーザーが実装計画を承認した。3 PR を順に merge する（stacked にしない）。各 PR の head で
  独立監査を行い、Issue は PR3 の merge 後確認を終えてから Close する。
- AC / INVAR、固定 inventory（INV-1〜6）と状態遷移（TR-1〜8）の正本は
  [Issue #1061](https://github.com/kukuri-app/kukuri/issues/1061)。
- 本番反映は別 Issue にまとめる。本 Issue の Close 条件に本番確認は含めない。

| PR | 範囲 | 状態 |
| --- | --- | --- |
| PR1 | ADR 0026 §8 / ADR 0022 追補、観測 envelope、CN の観測受付・取消・保持、T/R の合算と一括評価、wire 追従 | 実装中 |
| PR2 | client の提供トグル（任意文書への同意）・送信待ち・取消 | 未着手 |
| PR3 | CN の採用順位、6 経路と引用元の折りたたみ・再表示・著者例外 | 未着手 |

## 利用者決定（2026-09-17）

| 項目 | 決定 |
| --- | --- |
| 観測提供の同意 | CN の同意カタログの任意文書（slug `trust_observation_sharing`、`required: false`）に含め、client は CN ごとのトグルでその文書への同意・取消を行う |
| 既存分 | 有効化時に「既存のブロック/ミュートも送る」を選べる（既定は送らない） |
| 表示 | 6 経路で投稿を折りたたみ、理由・採用 CN・「表示する」を出す。作者詳細で常に表示する例外を設定・解除できる |
| PR 分割 | 3 PR を順に merge |

## 修正前の状態（基準 commit）

- `trust_user_read` は `UniformRelationWeight(1.0)` で T だけを返し、閲覧者で値が変わらない。
- CN に block / mute を受け付ける route・保存先が無い（`/v1/trust/observations` は 404）。
- `RelationStore` に複数 candidate の proximity を読む口が無い。
- 同意カタログに観測提供の文書種別が無い。

## PR1 の対応

| 条件 | 実装 | 検証 |
| --- | --- | --- |
| AC-1 / INVAR-1 / INVAR-3 | ADR 0026 §8.3 / §8.5・ADR 0022 追補。`kukuri-core` の `mute-observation` envelope と `parse_trust_observation`（署名・id・署名者 = subject を検証、block-edge を block 観測として受理）。`cn-core` migration `202609180001_trust_observations.sql`（観測・対象 revision・任意同意の取消）。受付は bearer = 署名者・必須同意・任意文書の現行版同意を observer 単位の advisory lock 下で確認してから保存。取消は本人認証だけで全削除 | core unit 4 件、`observation_intake_requires_matching_signer_and_sharing_consent`（拒否時の行数 0）、`observation_revocation_deletes_rows_and_blocks_intake` |
| AC-1（任意文書） | `cn-operator` の `LegalDocumentKind::TrustObservationSharing`（`ALL` に含めない、slug 固定、required 禁止）と本文生成。operator runbook に設定例 | `trust_observation_sharing_document_is_optional_and_published_only_when_configured`、`trust_observation_sharing_document_must_be_optional_with_fixed_slug`、`sharing_policy_slug_matches_operator_catalog` |
| AC-2 / TR-2 / TR-6 | `cn-trust::compose_relation_adjustment`（proximity 重み・下限 0.1・block 1.0 / mute 0.5 の max・半減期・上位 5 件の noisy-OR）。`(observed_at_ms, envelope_id)` が新しい観測だけ採用、未来時刻は拒否、保持 active 180 日 / revoked 30 日 | `cn-trust/tests/relation_adjustment.rs` 13 件、`observation_upsert_is_idempotent_and_ignores_stale`、`observation_retention_purges_expired` |
| AC-6 / AC-7 / INVAR-2 / INVAR-5 / TR-7 / TR-8 | T = 従来の `build_trust_read`（一様重み）、R は観測と proximity だけから算出、`apply_viewer_relation` が S = clamp(T + R) と `evaluation`（policy / trust / relation 版、期限、`hide_recommended`、理由）を付ける。`GET /v1/trust/users/{pubkey}` の `trust` を S に変更し、`POST /v1/trust/evaluations`（最大 100 件）を追加。`RelationStore::proximity_scores` を追加 | `trust_read_sums_absolute_and_viewer_relation`（A/C 比較、T 共通、解除で復帰、同意取消で除外、observer 非開示、pull 不変）、`relation_proximity_scores_match_pairwise_reads`、ArcadeDB の `arcadedb_relation_store_satisfies_shared_contracts` |
| AC-6（wire） | `TrustReadView.evaluation`（旧応答互換）、`TrustEvaluation*` 型、path / 安定コード。desktop-runtime の TS 生成、CLI の JSON schema、harness mock、`CommunityNodeAdvisoryPanel` は S を主値にし、内訳を「ノード共通の評価の内訳」として分けた | `trust_evaluation_wire_contract_is_optional_and_carries_no_observer`、`community_node_trust_relation_client_preserves_wire_contract_and_methods`、`CommunityNodeAdvisoryPanel.test.tsx` の 2 件 |

## 既知の制約（ADR 0026 §8 に記録）

- relation snapshot は `relation analyze` の上書き更新を版として使う。解析中の read で更新前後の proximity が混在しうる。
- revoked 観測は 30 日で削除するため、それより古い active envelope の再送は復活しうる。client の送信待ちは
  対象ごとに最新 1 件へ集約するので、通常の再送では起きない。
- 1 対象あたり評価に使う active 観測は新しい順に 200 件まで。

## 検証記録

- PR1: 実行結果は PR 本文に記録する。
