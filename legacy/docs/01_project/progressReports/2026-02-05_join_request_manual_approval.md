# join.request 手動承認/Rate Limit 実装
日付: 2026年02月05日

## 概要
- join.request 受信側で rate limit + pending 保存に切り替え、key.envelope の自動配布を停止。
- 承認/却下フロー（Tauri コマンド + フロント API + UI）を追加。
- 承認時のみ key.envelope を配布する P2P-only の運用に整理。

## 対応内容
- AccessControlService に join.request 受信の rate limit / pending 保存 / 承認・却下処理を追加。
- JoinRequestStore（SecureStorage ベース）とテストを追加。
- Tauri コマンドと DTO を追加し、CommunityNodePanel に pending 一覧と承認/却下 UI を実装。
- unit / integration テストを更新。

## 検証
- `./scripts/test-docker.ps1 rust`
- `./scripts/test-docker.ps1 ts`（act 警告 / useRouter 警告あり）
- `gh act --workflows .github/workflows/test.yml --job format-check`（git clone の some refs were not updated / pnpm approve-builds 警告）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（git clone の some refs were not updated / pnpm approve-builds 警告 / act・useRouter 警告 / docker_connectivity テストが長時間実行したが成功）
