# Community Nodes 実装タスク（ロードマップ）

最終更新日: 2026年01月23日

目的: `docs/03_implementation/community_nodes/*` の設計に基づき、M1-M5 の実装タスクを段階的に進められるように整理する。

## タスクファイル一覧

- M1: リポジトリ雛形 + Compose: `docs/01_project/activeContext/tasks/priority/community_nodes.md`
- M2: bootstrap/relay 統合（iroh-gossip + 永続化 + outbox + 39000/39001 + Access Control）: `docs/01_project/activeContext/tasks/priority/community_nodes_m2.md`
- M3: Index v1（Meilisearch）: `docs/01_project/activeContext/tasks/priority/community_nodes_m3.md`
- M4: Moderation v1（ルール）+ v2準備（LLM）: `docs/01_project/activeContext/tasks/priority/community_nodes_m4.md`
- M5: Trust v1（Apache AGE）: `docs/01_project/activeContext/tasks/priority/community_nodes_m5.md`

## 参照（設計）

- `docs/03_implementation/community_nodes/summary.md`（全体方針とマイルストーン）
- `docs/03_implementation/community_nodes/architecture_overview.md`（責務分割/データフロー）
- `docs/03_implementation/community_nodes/repository_structure.md`（構成案）
