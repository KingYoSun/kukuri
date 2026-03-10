# Community Nodes M5 Trust v1 完了
日付: 2026年01月24日

## 概要
- AGE を使った trust 計算（通報/コミュ濃度）と attestation 発行の v1 を実装。
- outbox 追従、再計算ジョブ、Admin/Console の運用導線まで通貫で整備。

## 対応内容
- cn_trust schema/migration と AGE graph 初期化を追加。
- cn-trust: outbox consumer、report/label/interaction 取り込み、score 計算、attestation 発行を実装。
- ジョブキュー/スケジュール（再計算）と trust worker を追加。
- User API に trust 参照（report-based/communication-density）を実装。
- Admin API/Console に trust ジョブ/スケジュール管理画面を追加。

## 検証
- `./scripts/test-docker.ps1 rust`（警告: `dead_code` など）
- `./scripts/test-docker.ps1 ts`（警告: `act(...)` 未ラップ、`useRouter` が `RouterProvider` 外）
- `gh act --workflows .github/workflows/test.yml --job format-check`（警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（警告: `git clone` の `some refs were not updated`、`pnpm approve-builds`、`useRouter must be used inside a <RouterProvider>`）

## 補足
- `gh act --workflows .github/workflows/test.yml --job native-test-linux` は 2 回タイムアウト後に 10 分枠で再実行して完了。
