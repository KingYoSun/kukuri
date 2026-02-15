# Issue #22 最終再監査（PR #23/#24/#25/#26 マージ後）

作成日: 2026年02月15日

## 監査前提

- strict checklist: `references/community-nodes-strict-audit-checklist.md` は存在しないため、Issue #5 fallback gate を適用。
  - fallback: https://github.com/KingYoSun/kukuri/issues/5#issuecomment-3900483686
- 監査対象:
  - `docs/03_implementation/community_nodes/*`
  - `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - `docs/01_project/activeContext/tasks/status/in_progress.md`
  - `docs/01_project/activeContext/tasks/completed/2026-02-15.md`

## Gate 判定（PASS/FAIL）

| Gate | 判定 | 根拠 |
|---|---|---|
| Gate1: codex CLI 実行品質 | PASS | 実コマンドで再監査（`rg`/`nl -ba`/`sed`/`gh api`）を実施し、placeholder 判定なし。 |
| Gate2: `GET /v1/bootstrap/hints/latest` 401/428/429 境界契約 | PASS | `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:424`, `:440`, `:477` に境界テストあり。 |
| Gate3: `spawn_bootstrap_hint_listener` 実DB受信経路 | PASS | 実装 `kukuri-community-node/crates/cn-user-api/src/lib.rs:354`、実DB通知テスト `:507` を確認。 |
| Gate4: `CommunityNodePanel.tsx` / `PostSearchResults.tsx` 対応テスト | PASS | `kukuri-tauri/src/tests/unit/components/settings/CommunityNodePanel.test.tsx:117`、`kukuri-tauri/src/tests/unit/components/search/PostSearchResults.test.tsx:130` を確認。 |
| Gate5: Issue #22 DoS 4タスク完了 | PASS | 下記エビデンステーブルのとおり、実装 + 契約/統合テストを確認。 |
| Gate6: 追加不足タスクの有無 | PASS | `community_nodes_roadmap.md` の未完チェックは 0 件（`rg -n "- \[ \]" ...` 該当なし）。 |

## エビデンステーブル（Issue #22 DoS 4タスク）

| 対象 | 実装/テストファイル | 確認コマンド | 結果 |
|---|---|---|---|
| per pubkey 同時保留数上限の実装 | `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:132` | `rg -n "max_pending_per_pubkey|PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED" ...` | pending 件数を transaction lock 下で判定し、超過時 `429 + PENDING_SUBSCRIPTION_REQUEST_LIMIT_REACHED` を返す実装を確認。 |
| per pubkey 同時保留数上限の契約シナリオ（3ケース） | `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:2718`, `:2781`, `:2840` | `rg -n "topic_subscription_pending_request_limit_contract_" ...` | under-limit 受理 / at-limit 拒否 / approve-reject 後再申請可能 の3シナリオが存在。 |
| node-level 同時取込 topic 上限の実装 | `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs:193`, `:242`, `:310` | `rg -n "enforce_node_subscription_topic_limit|NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED" ...` | 承認時に node-level 上限を強制し、超過時 `429 + NODE_SUBSCRIPTION_TOPIC_LIMIT_REACHED` を返す実装を確認。 |
| node-level 上限の回帰テスト（admin契約 + relay統合） | `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs:3564`, `:3680`, `kukuri-community-node/crates/cn-relay/src/integration_tests.rs:912` | `rg -n "node_topic_limit|node_subscription_limit_prevents_desired_topic_growth_when_over_limit" ...` | admin 側の承認拒否契約と relay 側の over-limit topic 選別回帰テストを確認。 |

## 仕様整合の確認

- DoS 要件定義: `docs/03_implementation/community_nodes/topic_subscription_design.md:154`-`:161`
  - per pubkey 同時保留数上限
  - node-level 同時取込 topic 数上限
- 上記要件と実装/テストの対応が成立していることを確認。

## 結論

- strict gate は **Gate1-6 すべて PASS**。
- 追加実装タスクは **0件**。
- Issue #22 はクローズ可能。
