# 2026年03月10日 foundation

## 実装済み
- root Cargo workspace と `cargo xtask` alias
- `next-core`, `next-store`, `next-transport`, `next-app-api`, `next-harness`
- `next/apps/desktop` の Linux-first shell
- `next-fast.yml`, `next-nightly.yml`

## 検証済み
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`

## 既知の制約
- `next-transport` の実 iroh 2-process handshake は local harness でまだ不安定
- Tauri backend binding は未着手
