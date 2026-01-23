# Community Nodes M1 実装着手準備

日付: 2026年01月23日

## 概要
- `kukuri-community-node` 配下に M1 の雛形を追加し、Compose + Caddy + 最小 API サービス起動まで到達できる構成を整備。
- `cn-cli` の migrate/seed/admin 操作と、User/Admin API の health/metrics/OpenAPI を最小実装で揃えた。

## 対応内容
- `kukuri-community-node/` を新設し、Cargo workspace と各 crate（`cn-cli` / `cn-core` / `cn-user-api` / `cn-admin-api` / `cn-relay` / `cn-bootstrap`）の骨組みを追加。
- `docker-compose.yml` と `Caddyfile`、`.env.example` を追加して core/admin/observability profile を定義。
- migrations に `cn_admin` / `cn_user` の最小スキーマを追加し、`cn-cli` に migrate/seed/admin bootstrap/reset-password を実装。
- User/Admin API に共通 middleware（request id/timeout/body limit/trace）と JSON logging を導入し、`/healthz`/`/metrics`/`/v1/openapi.json` を提供。
- relay/bootstrap は最小の HTTP daemon として `/healthz`/`/metrics` を提供。

## 検証
- `gh act --workflows .github/workflows/test.yml --job format-check`（成功）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（成功。`git clone` の `some refs were not updated` 警告あり）

## 補足
- admin-console は placeholder のため、UI 実装は次フェーズで追加予定。
