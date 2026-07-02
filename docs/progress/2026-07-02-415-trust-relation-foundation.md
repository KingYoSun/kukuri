# 2026-07-02 #415 community node trust / relation foundation（CommunityLocalTrust read surface）

参照: `docs/adr/0026-community-node-trust-relation-foundation.md`（§2 Decision / §6 #416 Decision）、
`.claude/plans/2026-07-02-issue-415-trust-relation-foundation.md`、
`docs/progress/2026-07-02-404-fail-closed-community-indexing.md`（index 真実源 = co-participation 観測元）

## 実装した範囲

`CommunityLocalTrust` capability の中身 = **trust（per-user 信頼度）+ relation（pairwise cluster
proximity）の 2 read** を foundation として実装した。trust / relation とも node-local **advisory**
であり、canonical（user identity / social graph）を所有・改変しない overlay。

- **`cn-trust`（新 crate。pure domain、DB / network 非依存）**
  - `TrustParams`: operator 可変パラメータ（`COMMUNITY_NODE_TRUST_W_ABS_NEGATIVE` 既定 2.0 /
    `..._W_ABS_POSITIVE` 既定 1.0 / `..._RELATIVE_HALF_LIFE_DAYS` 既定 30）。不正値は起動時 Err。
  - scoring（§6.2）: `trust = clamp(-1, 1, (w_abs*absolute + relative) / 2)`。絶対成分 =
    critical safety risk signal（減衰なし・relation 非依存・viewer 非依存）、相対成分 = 非 critical
    risk signal（半減期減衰 + relation 重み seam `RelationWeighting`）。appeal は供給層
    （#406 `trust_risk_inputs_from`）+ scoring 層の二重防御（pending 据え置き / accepted 除外）。
  - `TrustReadView`: 絶対 / 相対 / 合成 + 寄与 signal ごとの根拠（issuer / basis / confidence /
    visibility / expiry / appeal / decay / relation 重み / 実効寄与）。断定ラベル・閾値は持たない。
  - relation（§6.1）: graph-store 抽象 `RelationStore`（`upsert_edge` / `pairwise_proximity` /
    `neighbors` / `cluster_of` + `set_cluster`）、backend 非依存の proximity 合成
    `proximity_from_features`（feature 内訳つき）、in-memory 実装、共有 contract スイート
    （`testing` feature。in-memory / ArcadeDB に同一契約を課す）。
  - cross-node 開示（§6.3）: `cross_node_trust_disclosure` = **confirmed（known-hash /
    provider-verdict）な絶対成分のみ**。suspected / 相対成分 / relation は返さない。開示値は
    開示可能な signal のみから再計算（Local signal の存在を漏らさない）。
  - seam: `ObservedSignal`（観測者つき node-local 観測）は**型のみ**。producer は非決定論的
    moderation（ADR 0028 系）実装まで存在しない。`FEATURE_FOLLOW_PROJECTION` も key 予約のみ。
- **`cn-core`** — migration `202607020002_trust_relation.sql`
  - `cn_trust.relation_optouts`: 「見えない」opt-out の真実源（可逆 = 行削除、冪等）。
    **trust には影響しない**（trust read は本テーブルを見ない）。
  - `co_participation.rs`: index 真実源（`cn_index.index_entries`、allow verdict のみ）からの
    共起集計。`CoParticipationSource` trait（Pg / in-memory 同一セマンティクス）。
    **`public_topic` scope のみ**を集計し、private channel 由来は構造的に relation へ入らない。
  - `trust_inputs.rs` の型（`TrustRiskInput` 等）は cn-trust へ移動し再エクスポート（互換維持）。
- **`cn-indexer`**
  - `ArcadeDbRelationGraph`: `RelationStore` の ArcadeDB 実装。index 投影と同じインスタンスに
    相乗り（#413 方針）。クエリは **Cypher**（neo4j 互換の scale path、§6.1）。edge feature は
    `features_json` 文字列 property（neo4j も map property を持たないため可搬）。
  - `relation_worker.rs`: co-participation 集計 → feature 点数化（共有 topic 数 / 共起 events）→
    `upsert_edge` + cluster 帰属（**dominant shared topic** の初期実装）。batch・冪等・**非常駐**。
- **`cn-cli`**: `relation analyze`（`--limit`）。schema 冪等作成 + batch 解析の手動実行。
- **`cn-user-api`** — すべて `COMMUNITY_NODE_TRUST_READ_ENABLED`（**既定 false = 404**）で gate:
  - `GET /v1/trust/users/{pubkey}`: viewer 相対 trust read。**viewer = bearer identity に固定**
    （challenge への鍵署名で検証済み。他人視点を騙る手段が無い）。認証 + consent + rate limit。
  - `GET /v1/trust/pull/{pubkey}`: cross-node pull。匿名 = `Public` visibility のみ、bearer で
    subscriber 認証済み = `SubscribedNodes` も。confirmed 絶対成分のみ根拠つき。
  - `GET /v1/relation/users/{target}` / `GET /v1/relation/neighbors`: pairwise proximity（根拠つき）
    と近接近傍。opt-out 済み user は双方から消える（opt-out 状態自体は 404 で漏らさない）。
  - `PUT /v1/relation/optout` / `DELETE /v1/relation/optout`: 自分自身のみ・可逆。
  - relation を使って index / search / discovery の結果集合を削る経路は**存在しない**
    （auto-suppression しない。relation read は情報を返すだけ）。

## ADR 0026 contract ↔ テスト対応

| contract | テスト |
|---|---|
| `trust_is_not_single_absolute_scalar` | cn-trust `tests/trust_scoring.rs` 同名 |
| `trust_separates_absolute_and_relative_indicators` | 同上 同名 |
| `trust_absolute_indicators_not_relation_weighted` | 同上 同名 |
| `trust_relative_indicators_are_relation_weighted` | 同上 同名 |
| `trust_resists_mass_report_bombing` | 同上 同名 + cn-user-api `trust_read_returns_components_with_basis_and_ignores_reports`（通報は入力にならない構造的保証） |
| `trust_absolute_negative_is_weighted_double` | 同上 同名 |
| `trust_composition_weights_are_operator_tunable` | 同上 同名 |
| `trust_reflects_risk_signals_split_by_category` | 同上 同名 + cn-core `tests/trust_inputs.rs`（#406） |
| `trust_does_not_own_or_mutate_user_identity` | read-only surface（書き込み口は opt-out のみ、canonical 非接触）。cn-indexer `relation_worker_is_idempotent_and_does_not_mutate_index_source` |
| `trust_read_is_explainable_with_basis` | cn-trust 同名 + cn-user-api（basis の issuer / basis / visibility を assert） |
| `trust_is_clamped_to_unit_interval` | cn-trust 同名 |
| `trust_absolute_component_does_not_decay` | cn-trust 同名 |
| `trust_relative_component_decays_over_time` | cn-trust 同名 |
| `trust_appeal_pending_holds_contribution` | cn-trust 同名 + cn-core `disputed_appeal_holds_contribution` |
| `trust_appeal_accepted_excludes_contribution` | cn-trust 同名 + cn-core `cleared_appeal_excludes_contribution` |
| `cross_node_pull_discloses_only_confirmed_absolute_component` | cn-trust `tests/cross_node_disclosure.rs` 同名 + cn-user-api 同名 |
| `viewer_relative_read_requires_authenticated_viewer` | cn-user-api `viewer_relative_read_requires_authenticated_viewer` |
| `relation_is_pairwise_cluster_proximity` | cn-trust `tests/relation_contracts.rs` 同名（共有スイート、ArcadeDB 実装にも `cn-indexer tests/relation_contracts.rs` で適用） |
| `relation_does_not_mutate_social_graph_canonical` | `RelationStore` に canonical への書き込み口が無い（型レベル）+ cn-indexer `relation_worker_is_idempotent_and_does_not_mutate_index_source` |
| `relation_read_is_explainable` | cn-trust `relation_read_is_explainable` + cn-user-api（basis assert） |
| `relation_visibility_choice_is_user_controlled_and_reversible` | cn-core `tests/relation_optouts.rs` 同名 + cn-user-api `relation_read_optout_and_no_auto_suppression` |
| `relation_does_not_auto_suppress_cross_cluster_content` | cn-user-api 同上（relation が index 応答へ介在しない構造 + edge 無し = 404 は情報なしであって抑制でない） |
| `relation_read_requires_authenticated_viewer` | cn-user-api `viewer_relative_read_requires_authenticated_viewer` |
| `relation_opt_out_hides_from_others_relation_and_discovery` | cn-user-api `relation_read_optout_and_no_auto_suppression`（relation read + neighbors 双方） |
| `relation_defaults_local_and_not_cross_node_pullable` | relation の cross-node endpoint が存在しない（構造）+ cn-trust `relative_component_never_crosses_node_boundary` |
| `relation_private_channel_signal_stays_local_and_scoped` | cn-core `co_participation_pairs_from_public_topics_only_*`（集計が channel scope を除外）+ cn-indexer worker テスト（private のみの共起ペアが graph に無い） |

opt-out が trust に影響しないこと: cn-core `relation_optout_does_not_affect_trust_inputs` +
cn-user-api（opt-out 後も trust read が変わらない）。

## デプロイ順序（重要）

`ensure_database_ready`（`RequireReady`）が新テーブル `cn_trust.relation_optouts` の存在を要求する。
**新バイナリの RequireReady 起動より前に migration（Prepare / migrate 手順）を適用**すること
（#405 / #413 / #404 と同じ fail-closed 運用）。

新 env（`.env.community-node.example` に追記済み）:
- `COMMUNITY_NODE_TRUST_READ_ENABLED`（cn-user-api、**既定 false**）。有効化する場合は
  `COMMUNITY_NODE_ARCADEDB_*`（relation graph。cn-indexer と同一値）も渡し、
  `cn-cli relation analyze` で relation graph を構築しておく。
- `COMMUNITY_NODE_TRUST_W_ABS_NEGATIVE` / `..._W_ABS_POSITIVE` / `..._RELATIVE_HALF_LIFE_DAYS`
  （operator 可変。未設定は ADR 0026 §6.2 の初期値）。

## 維持した境界（本 PR に含まない）

- `CommunityLocalTrust` の `Availability::Planned` → 昇格（issue 記載どおり実装・テスト後に別途判断）。
  read flag の既定有効化・relation 解析 worker の常駐化も同判断に紐づけて据え置き。
- 相対成分の入力は**非決定論的 moderation（ADR 0028 / #420 系）実装まで** risk signal のみ。
  観測者つき観測（`ObservedSignal`）と relation 重みの実運用値はその時点で結線する
  （seam は scoring 層でテスト済み）。
- follow projection（ADR 0013）の CN 側観測 = `FEATURE_FOLLOW_PROJECTION` の producer。
- private channel の channel メンバー可視 relation（foundation では graph に入れない fail-closed）。
- clustering の高度化（community detection / 重み実測 / pairwise 計算・保存コスト最適化、ADR 0026 §4）。
- neo4j adapter 本体（Cypher 互換のクエリ形と `RelationStore` trait までが本 issue）。
- P2P 層での cross-node pull 実配信（HTTP read 境界まで）。client UI。

## 検証

- `cargo test -p kukuri-cn-trust`（scoring 15 / relation 6 / cross-node 3）: pass。
- `cargo test -p kukuri-cn-indexer --test relation_contracts`: pass（ArcadeDB は
  `KUKURI_CN_RUN_ARCADEDB_TESTS=1` + live ArcadeDB で共有スイートを実行）。
- `cargo clippy`（cn-trust / cn-core / cn-indexer / cn-user-api / cn-cli, all-targets,
  `-D warnings`）: clean。`cargo fmt --check`: clean。
- `cargo xtask cn-check`: pass。
- `cargo xtask cn-test`（Postgres + Redis harness, `KUKURI_CN_RUN_INTEGRATION_TESTS=1`）: pass
  （cn-core relation_optouts / co_participation、cn-user-api trust_relation の integration 含む）。
