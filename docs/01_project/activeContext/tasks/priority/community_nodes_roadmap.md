# Community Nodes 実装タスク（ロードマップ）

最終更新日: 2026年01月26日

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
- [x] 計画更新: `docs/01_project/activeContext/community_node_plan.md` の Node HTTP API パスを現行実装に合わせて更新する（`/v1/bootstrap/*`、`/v1/reports`、`/v1/keys/envelopes`、`/v1/search`）
- [x] テスト: `cn-relay`/`cn-bootstrap`/`cn-admin-api`/`cn-kip-types` の統合・契約テストと、User API 主要エンドポイントの契約テストを追加する

## 参照（設計）

- `docs/03_implementation/community_nodes/summary.md`（全体方針とマイルストーン）
- `docs/03_implementation/community_nodes/architecture_overview.md`（責務分割/データフロー）
- `docs/03_implementation/community_nodes/repository_structure.md`（構成案）
