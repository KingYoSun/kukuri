# Community Nodes M2 完了

日付: 2026年01月24日

## 概要
- Community Nodes M2 の実装範囲を完了し、admin console と運用設定の整備まで到達。
- relay/bootstrap/user/admin API の統合を前提に、運用 UI と compose 設定を仕上げた。

## 対応内容
- `kukuri-community-node/apps/admin-console` に管理 UI（Dashboard/Services/Subscriptions/Policies/Audit & Health）を実装。
- 管理 UI で Admin API の認証・設定更新・購読審査・ポリシー公開・監査ログ閲覧を可能にした。
- `kukuri-community-node/docker-compose.yml` に admin-console 起動・Node Key 用ボリュームを追加。
- `kukuri-community-node/.env` / `.env.example` を更新し、Node Key/Personal Data/relay P2P などの設定を反映。

## 検証
- `./scripts/test-docker.ps1 ts`（成功。既存テストで `act(...)` 警告と `useRouter` 警告あり）
- `./scripts/test-docker.ps1 rust`（成功。`dead_code` の警告あり）
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功。`git clone` の `some refs were not updated` 警告あり）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。`git clone` の `some refs were not updated` 警告あり）

## 補足
- admin-console は `/admin` 配下で起動する前提（Caddy の `/admin` ルーティング）。
