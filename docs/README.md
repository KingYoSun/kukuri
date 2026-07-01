# kukuri docs

## 目的
- 現行 kukuri 実装に必要な情報だけを置く。
- 仕様は ADR、実行手順は runbook、状態は progress に分ける。

## 優先参照順
1. `docs/progress/2026-04-16-mvp-builder-preview-plan.md`
2. `docs/progress/2026-03-10-foundation.md`
3. `docs/progress/2026-03-24-shell-ui-production-migration.md`
4. `docs/runbooks/dev.md`
5. `docs/runbooks/mvp-user-quickstart.md`
6. `docs/runbooks/mvp-troubleshooting.md`
7. `docs/adr/0001-linux-first-mvp.md`
8. `docs/adr/0007-windows-desktop-support.md`
9. `docs/adr/0008-dht-discovery-data-classification.md`
10. `docs/adr/0009-community-node-relay-auth-data-classification.md`
11. `docs/adr/0014-uiux-dev-flow.md`
12. `DESIGN.md`（root・ビジュアル仕様）
13. `harness/scenarios/`
14. `docs/adr/0003-image-post-data-classification.md`
15. `docs/adr/0004-video-post-data-classification.md`

## 現在の対象
- `desktop + core + store + docs-sync + blob-service + desktop-runtime + cn-* + harness`
- desktop target は Linux / Windows
- current connectivity scope は `static-peer + seeded DHT + community-node connectivity/auth`
- current product scope には `social graph v1 + private channel audience v1` を含む
- root 実行入口は `cargo xtask ...`
- 日常 validation は `cargo xtask check` + `cargo xtask test`
- browser-level UI change は `cargo xtask desktop-ui-check`
- community-node / Postgres slice は `cargo xtask cn-check` + `cargo xtask cn-test`
- targeted rerun は `cargo xtask rust-check|rust-test|tauri-check|desktop-lint|desktop-test|desktop-storybook|desktop-browser-test`
- 新 feature 着手前に `docs/adr/0002-feature-data-classification-template.md` を埋める。

## Ops
- community node GCP Terraform デプロイ（deployment profile: low-cost / managed-db / ha）: `docs/runbooks/community-node-gcp-terraform.md`（実装は `infra/terraform/`）

## Architecture
- P2P-first community node の責任境界: `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（operator docs / safety / report routing の共通前提）
- moderation event / safety advisory の trust semantics + deterministic (CSAM / known-hash) critical safety: `docs/adr/0027-deterministic-moderation-critical-safety.md`（optional trust input であり network-wide command ではないことを固定。旧 `community-node-critical-safety.md` / `moderation-event-trust-semantics.md` を集約）
- community node trust / relation foundation: `docs/adr/0026-community-node-trust-relation-foundation.md`
- default community node 依存低減ロードマップ: `docs/architecture/default-community-node-dependency-reduction.md`（default node は onboarding infrastructure であり network-wide authority ではない）

## Legal
- app-level 利用規約 / プライバシーポリシー（client 自体への同意。per-node consent とは別建て）: `docs/legal/terms-of-service.md` / `docs/legal/privacy-policy.md`（canonical SSoT。アプリ内表示は i18n `legal` namespace がミラー）
- feature data classification: `docs/legal/app-consent-data-classification.md`

## UI/UX
- flow: `docs/adr/0014-uiux-dev-flow.md`
- visual spec: `DESIGN.md`（root）
- migration plan: `docs/progress/2026-03-24-shell-ui-production-migration.md`
- accepted review records: `docs/ui-reviews/`
