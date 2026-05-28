# ADR 0001: Linux-first MVP

## Status
Accepted

## Decision
- 現行 kukuri 実装の MVP は Linux を required target にする。
- スコープは `desktop + core + store + static-peer transport + harness` に限定する。
- Windows、DHT discovery、community-node は cutover 後の拡張フェーズへ送る。
- root の公式 entrypoint は `cargo xtask ...` に統一し、`doctor|check|test|e2e-smoke` を基本導線、granular rerun は同じ `cargo xtask <subcommand>` 配下に置く。

## Consequences
- fast lane は deterministic な harness と fake transport を優先する。
- `docs` は短い pointer / ADR / runbook だけに保つ。
