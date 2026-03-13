# kukuri docs

## 目的
- 現行 kukuri 実装に必要な情報だけを置く。
- 仕様は ADR、実行手順は runbook、状態は progress に分ける。
- `legacy/docs` は参照専用の履歴として扱う。

## 優先参照順
1. `docs/adr/0001-linux-first-mvp.md`
2. `docs/runbooks/dev.md`
3. `docs/progress/2026-03-10-foundation.md`
4. `harness/scenarios/`

## 現在の対象
- `desktop + core + store + static-peer transport + harness`
- required target は Linux のみ
- root 実行入口は `cargo xtask ...`
- 新 feature 着手前に `docs/adr/0002-feature-data-classification-template.md` を埋める。
