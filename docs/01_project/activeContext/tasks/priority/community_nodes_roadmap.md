# Community Nodes 実装タスク（ロードマップ）

最終更新日: 2026年02月03日

目的: `docs/03_implementation/community_nodes/*` の設計に基づき、M0-M5 とクライアント実装のタスクを段階的に進められるように整理する。

## タスクファイル一覧

- M0: KIP-0001 仕様固定 + kip_types 基盤（新規）: `docs/01_project/activeContext/tasks/priority/community_nodes_m0.md`
- M1: リポジトリ雛形 + Compose: `docs/01_project/activeContext/tasks/priority/community_nodes.md`
- M2: bootstrap/relay 統合（iroh-gossip + 永続化 + outbox + 39000/39001 + Access Control）: `docs/01_project/activeContext/tasks/priority/community_nodes_m2.md`
- M3: Index v1（Meilisearch）: `docs/01_project/activeContext/tasks/priority/community_nodes_m3.md`
- M4: Moderation v1（ルール）+ v2準備（LLM）: `docs/01_project/activeContext/tasks/priority/community_nodes_m4.md`
- M5: Trust v1（Apache AGE）: `docs/01_project/activeContext/tasks/priority/community_nodes_m5.md`
- Client: ノード採用/label・attestation適用/鍵管理/暗号化投稿（新規）: `docs/01_project/activeContext/tasks/priority/community_nodes_client.md`

## 欠落タスク（対応済み）

- [x] M0: `docs/kips/` に KIP-0001 v0.1 を追加し、`cn-kip-types` に共通の kind/tag/exp/署名検証を実装する
- [x] クライアント: ノード採用 UI、label/attestation 適用、key.envelope 受理と鍵保管、scope 別暗号投稿/復号を実装する
- [x] 計画更新: `docs/01_project/activeContext/community_node_plan.md` の Node HTTP API パスを現行実装に合わせて更新する（`/v1/bootstrap/*`、`/v1/reports`、`/v1/search`）。Access Control は P2P-only を正とする
- [x] テスト: `cn-relay`/`cn-bootstrap`/`cn-admin-api`/`cn-kip-types` の統合・契約テストと、User API 主要エンドポイントの契約テストを追加する

## 実ノードE2E/統合テスト拡充（計画）

- [x] E2E 用に `kukuri-community-node` を起動する Docker 経路を追加し、実ノードの base URL をテストへ配布する（`docker-compose.test.yml`/`scripts/test-docker.*`/`SCENARIO` を整理）。
- [x] 実ノードの DB/Meilisearch に投入する E2E シード（ユーザー/トピック/投稿/label/trust）を作成し、テスト前に投入/後に掃除できるようにする。
- [x] 実ノード認証フロー（challenge/verify）を通すヘルパーを追加し、`community-node` 設定/認証/同意取得の E2E を実ノードで再実行する。
- [x] （legacy）invite.capability と key.envelope を実ノードで発行できるテストヘルパー（`cn-cli`）を整備し、招待適用/鍵同期/暗号化投稿の E2E を追加した（**P2P-only 移行のため参考扱い**）。
- [x] P2P-only: `access_control_issue_invite`/`access_control_request_join` を使った **invite共有→join.request→key.envelope→暗号化投稿** の E2E を追加する。
- [x] label/attestation/trust を実ノードで発行し、PostCard のラベル/信頼バッジ表示まで検証する E2E を追加する。
- [x] search/index の実データを投入し、検索UI（サジェスト/ページング/0件）と community node search API の連携を E2E で確認する。
- [x] bootstrap/relay 実ノードのエンドポイントを使った P2P 連携確認（list_bootstrap_nodes/services）を E2E に追加する。
- [x] 実ノード E2E のログ/artefact を `test-results/community-node-e2e` と `tmp/logs/community-node-e2e` に集約し、Runbook/CI の artefact 収集に追加する。
- [x] 実ノード E2E を Nightly/CI（`test.yml`/`nightly.yml`）へ組み込み、実行条件と所要時間を明記する。
  - CI: `test.yml` の `desktop-e2e`（push/pull_request: main/develop + workflow_dispatch）。`./scripts/test-docker.ps1 e2e-community-node` を使用。
  - Nightly: `nightly.yml` の `community-node-e2e`（cron: `0 15 * * *` + workflow_dispatch）。`./scripts/test-docker.sh e2e-community-node` を使用。
  - 所要時間目安: キャッシュあり 15〜25分、キャッシュなし 30〜45分（Docker build + seed + E2E 実行）。

## 未実装/不足事項（2026年02月02日 追記）

- [x] クライアントのノード採用設定を **複数ノード/role別** に拡張し、label/trust/search/bootstrap を採用ノード単位で切替できるようにする（UI/Store/DTO/Handler を含む）。
- [x] trust.anchor(39011) の発行/保存/適用/UI を実装し、採用attester切替で表示が変わることを確認する（M5受入条件の未達）。
- [x] Access Control の **epochローテ/追放** フローを実装する（epoch++ と残留者への key.envelope 再配布、ローテ操作の CLI/サービス化）。
- [x] `access_control` は P2P-only を正とし、User API の invite/keys エンドポイントは廃止（実装・UI・ドキュメントを統一）。
- [x] 通報 UI（reason 選択 + submit）を PostCard などに追加し、`community_node_submit_report` を利用した E2E を追加する。
- [ ] label/attestation のクライアント側検証（署名/exp/採用ノード）を導入し、未採用ノード由来のデータは無視する。
- [ ] テスト不足の補完:
  - [ ] `cn-kip-types`: 39001/39005/39010/39011/39020 の検証テスト追加。
  - [ ] `cn-user-api`: `/v1/bootstrap/*` `/v1/reports` `/v1/search` の契約テスト追加。
  - [ ] `cn-admin-api`: login/logout など主要エンドポイントの契約テスト追加。
  - [ ] `kukuri-tauri`: `CommunityNodeHandler` の単体/契約テスト追加。

## 参照（設計）

- `docs/03_implementation/community_nodes/summary.md`（全体方針とマイルストーン）
- `docs/03_implementation/community_nodes/architecture_overview.md`（責務分割/データフロー）
- `docs/03_implementation/community_nodes/repository_structure.md`（構成案）
