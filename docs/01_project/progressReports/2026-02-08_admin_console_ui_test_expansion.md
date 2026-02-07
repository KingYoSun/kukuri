# Admin Console UI テスト拡充レポート

作成日: 2026年02月08日
対象: `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md`

## 概要

`Admin Console` の UI テストを拡充し、以下 6 ページで主要操作と表示崩れ防止を検証できるようにした。

- `Dashboard`
- `Services`
- `Subscriptions`
- `Policies`
- `Trust`
- `Audit Logs`

## 実装内容

- `kukuri-community-node/apps/admin-console/src/pages/DashboardPage.test.tsx`
  - 集計カードとサービス一覧表示、`Refresh` による再取得を検証。
- `kukuri-community-node/apps/admin-console/src/pages/ServicesPage.test.tsx`
  - 設定 JSON バリデーション、`Save config` の API 呼び出しを検証。
- `kukuri-community-node/apps/admin-console/src/pages/SubscriptionsPage.test.tsx`
  - 申請承認/却下、購読トグル、プラン作成/更新、購読更新、利用量取得を検証。
- `kukuri-community-node/apps/admin-console/src/pages/PoliciesPage.test.tsx`
  - ポリシー作成、編集、`Publish`、`Make current` を検証。
- `kukuri-community-node/apps/admin-console/src/pages/TrustPage.test.tsx`
  - ジョブ投入、スケジュール更新、`Run now`、`Refresh` を検証。
- `kukuri-community-node/apps/admin-console/src/pages/AuditPage.test.tsx`
  - ヘルス表示、監査ログ表示、フィルタ条件反映を検証。

## 検証結果

- `docker run --rm -e CI=true -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node/apps/admin-console node:20-bookworm bash -lc "set -euo pipefail; corepack enable; pnpm install --frozen-lockfile; pnpm vitest run src/pages/DashboardPage.test.tsx src/pages/ServicesPage.test.tsx src/pages/SubscriptionsPage.test.tsx src/pages/PoliciesPage.test.tsx src/pages/TrustPage.test.tsx src/pages/AuditPage.test.tsx"`
  - 成功: `6 passed`
- `./scripts/test-docker.ps1 ts`
  - 成功: `92 files / 822 passed / 6 skipped`
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 成功。ログ: `tmp/logs/gh-act-format-check-20260208-041927.log`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - 成功。ログ: `tmp/logs/gh-act-native-test-linux-20260208-042040.log`

## 備考

- `gh act` 実行中に `some refs were not updated` が表示されるが、既知の非致命警告でありジョブは成功。
