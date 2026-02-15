# Issue #22 Community Nodes 再監査（b865ec92 起票分）

作成日: 2026年02月15日

## 監査前提

- 対象コミット: `b865ec92115efffb97768c1ed009292104ce1aeb`
- 厳格チェックリスト: `references/community-nodes-strict-audit-checklist.md` はリポジトリ内に存在しないため、Issue #5 strict gate コメントを fallback として適用。
  - fallback: https://github.com/KingYoSun/kukuri/issues/5#issuecomment-3900483686

## Gate 判定（PASS/FAIL）

| Gate | 判定 | 根拠 |
|---|---|---|
| Gate1: codex CLI 実行品質 | PASS | 本監査で実コマンドを連続実行し、エビデンスを取得。 |
| Gate2: `GET /v1/bootstrap/hints/latest` 401/428/429 境界契約テスト | PASS | `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:424`, `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:440`, `kukuri-community-node/crates/cn-user-api/src/bootstrap.rs:477` に境界テストあり。 |
| Gate3: `spawn_bootstrap_hint_listener` 実DB受信経路テスト | PASS | 実装 `kukuri-community-node/crates/cn-user-api/src/lib.rs:351`、テスト `kukuri-community-node/crates/cn-user-api/src/lib.rs:504` を確認。 |
| Gate4: `CommunityNodePanel.tsx` / `PostSearchResults.tsx` テスト有無 | PASS | `kukuri-tauri/src/tests/unit/components/settings/CommunityNodePanel.test.tsx:117`、`kukuri-tauri/src/tests/unit/components/search/PostSearchResults.test.tsx:130` を確認。 |
| Gate5: commit `b865ec9` 起点タスクの完了判定 | FAIL | DoS上限要件が未実装（下記 Evidence）。 |
| Gate6: 追加起票要否（b865 起点タスクに加えて不足があるか） | PASS | 追加不足は確認されず、DoS 4タスクで監査差分を網羅。 |

## Evidence（DoS 4タスク未完）

| 対象 | 実装ファイル | 確認コマンド | 判定 |
|---|---|---|---|
| 申請同時保留数上限（per pubkey） | `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs:87`、`kukuri-community-node/crates/cn-user-api/src/billing.rs:59` | `rg -n "check_topic_limit\(|SELECT COUNT\(\*\) FROM cn_user\.topic_subscriptions" ...` | `check_topic_limit` は active 件数のみを参照し、pending 件数上限判定がないため FAIL |
| 申請同時保留数の契約テスト | `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs` | `rg -n "pending.*limit|同時保留|TOO_MANY|TOPIC_REQUEST" ...` | 同時保留上限を固定する契約テストが見当たらず FAIL |
| node-level 同時取込 topic 上限 | `kukuri-community-node/crates/cn-admin-api/src/subscriptions.rs:209` | `rg -n "INSERT INTO cn_admin\.node_subscriptions|approve_subscription_request" ...` | 承認時に無制限 upsert（上限拒否なし）のため FAIL |
| node-level 上限の回帰テスト | `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`、`kukuri-community-node/crates/cn-relay/src/integration_tests.rs` | `rg -n "node.*limit|上限|reject.*subscription" ...` | 上限超過拒否を固定する契約/統合テストが見当たらず FAIL |

補足（仕様根拠）:
- `docs/03_implementation/community_nodes/topic_subscription_design.md:154` 以降に DoS 必須要件として「申請の同時保留数上限」「node-level の同時取込 topic 数上限」が明記されている。

## 反映内容

- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
  - 2026年02月15日 調査追記の2タスク（OpenAPI反映 / E2E skip false-green 防止）を完了化。
  - 2026年02月15日 再調査追記として DoS 4タスクを未完で再起票。
- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - Issue #22 監査結果と次アクション（DoS 4タスク）を追記。
- `docs/01_project/activeContext/tasks/completed/2026-02-15.md`
  - Issue #22 監査完了エントリを追記。

## 結論

- commit `b865ec9` 起点の DoS タスクは **未完**。
- タスクの粒度は十分で、**追加タスクは不要**（現時点では 4 タスクで実装差分を網羅）。
