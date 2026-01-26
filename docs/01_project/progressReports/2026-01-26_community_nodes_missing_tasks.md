# Community Node 欠落タスク完了
日付: 2026年01月26日

## 概要
- KIP-0001 v0.1 と cn-kip-types の検証基盤を追加。
- クライアントの Community Node 設定/鍵同期/暗号投稿/label・trust 適用を実装。
- Node HTTP API パス更新と契約テストを追加。

## 対応内容
- `docs/kips/KIP-0001.md` を追加し、計画/ロードマップの参照を更新。M0/Client タスクファイルを追加。
- cn-kip-types に kind/tag/exp/署名検証を実装し、代表ケースのユニットテストを追加。
- cn-relay/cn-bootstrap/cn-admin-api/User API の契約・統合テストを追加。
- Tauri 側に Community Node 設定/認証/鍵同期/招待処理/label・trust 取得のコマンドと UI を追加。
- 投稿暗号化/復号/プレースホルダー表示と Post 関連テストを更新。

## 検証
- `./scripts/test-docker.ps1 lint -NoBuild`
- `./scripts/test-docker.ps1 rust -NoBuild`
- `./scripts/test-docker.ps1 ts`（警告: `act(...)` 未ラップ、`useRouter` が `RouterProvider` 外）
- `gh act --workflows .github/workflows/test.yml --job format-check`（警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（2回タイムアウト。`docker_connectivity` テストが 60 秒超で実行中のまま停止）

## 補足
- `native-test-linux` の完走にはタイムアウト枠の拡張が必要。
