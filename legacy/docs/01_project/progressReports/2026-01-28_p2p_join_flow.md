# P2P join フロー実装
日付: 2026年01月28日

## 概要
- join.request(39022) の型/検証を cn-kip-types に追加。
- Tauri に P2P join 用 AccessControlService/コマンドを追加し、invite共有→join.request→key.envelope 受領を実装。
- AccessControlService のユニットテストを追加。
- E2E/テスト文言を P2P-only 方針に合わせて整理（legacy E2E に注記追加）。

## 変更点
- `kukuri-community-node/crates/cn-kip-types/src/lib.rs`
- `kukuri-tauri/src-tauri/src/application/services/access_control_service.rs`
- `kukuri-tauri/src-tauri/src/presentation/commands/access_control_commands.rs`
- `kukuri-tauri/src-tauri/src/presentation/dto/access_control_dto.rs`
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`
- `docs/01_project/progressReports/2026-01-28_community_node_invite_e2e.md`
- `docs/01_project/activeContext/tasks/completed/2026-01-28.md`

## テスト
- `./scripts/test-docker.ps1 rust`（警告: `access_control_service.rs` の unused field）
- `gh act --workflows .github/workflows/test.yml --job format-check`（警告: git clone の `some refs were not updated` / pnpm の build script notice）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（警告: unused field / React `act(...)` warning）
