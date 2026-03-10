# 2026年03月10日 foundation

## 実装済み
- root Cargo workspace と `cargo xtask` alias
- `next-core`, `next-store`, `next-transport`, `next-app-api`, `next-harness`
- `next/apps/desktop` の Linux-first shell
- `next-fast.yml`, `next-nightly.yml`
- `next-transport` は公式 `iroh-gossip` example / docs に寄せて `receiver.joined()` ベースの join gating を導入
- `next-transport` に low-level baseline test を追加し、wrapper 依存の問題と `iroh-gossip` 本体の問題を分離できるようにした

## 検証済み
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`

## 既知の制約
- `next-transport` は ticket からの direct connect と 2-process gossip roundtrip を required に昇格済み
- Tauri backend binding は未着手
