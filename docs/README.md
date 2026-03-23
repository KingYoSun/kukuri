# kukuri docs

## 目的
- 現行 kukuri 実装に必要な情報だけを置く。
- 仕様は ADR、実行手順は runbook、状態は progress に分ける。
- `legacy/docs` は参照専用の履歴として扱う。

## 優先参照順
1. `docs/progress/2026-03-10-foundation.md`
2. `docs/progress/2026-03-24-shell-ui-production-migration.md`
3. `docs/runbooks/dev.md`
4. `docs/adr/0001-linux-first-mvp.md`
5. `docs/adr/0007-windows-desktop-support.md`
6. `docs/adr/0008-dht-discovery-data-classification.md`
7. `docs/adr/0009-community-node-relay-auth-data-classification.md`
8. `docs/adr/0014-uiux-dev-flow.md`
9. `docs/DESIGN.md`
10. `harness/scenarios/`
11. `docs/adr/0003-image-post-data-classification.md`
12. `docs/adr/0004-video-post-data-classification.md`

## 現在の対象
- `desktop + core + store + docs-sync + blob-service + desktop-runtime + cn-* + harness`
- desktop target は Linux / Windows
- current connectivity scope は `static-peer + seeded DHT + community-node connectivity/auth`
- current product scope には `social graph v1 + private channel audience v1` を含む
- root 実行入口は `cargo xtask ...`
- 新 feature 着手前に `docs/adr/0002-feature-data-classification-template.md` を埋める。

## UI/UX
- flow: `docs/adr/0014-uiux-dev-flow.md`
- rules: `docs/DESIGN.md`
- migration plan: `docs/progress/2026-03-24-shell-ui-production-migration.md`
- accepted review records: `docs/ui-reviews/`
