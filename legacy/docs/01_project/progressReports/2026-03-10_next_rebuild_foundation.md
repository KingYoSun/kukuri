# next rebuild foundation（2026年03月10日）

## 概要
- `next/` 配下に Linux-first の再構築 workspace を追加した。
- root から `cargo xtask doctor|check|test|e2e-smoke` を実行できるようにした。
- MVP 対象を `core/store/transport/harness/desktop shell` に限定した。

## 追加した内容
- Rust workspace: `next-core`, `next-store`, `next-transport`, `next-app-api`, `next-harness`, `xtask`
- NIP-01/NIP-10 契約テスト、SQLite 永続化、static-peer transport、scenario runner
- `next/apps/desktop` の React shell と frontend smoke test
- `next-fast.yml` / `next-nightly.yml` による Linux-first CI の土台

## 残課題
- `cargo xtask check/test/e2e-smoke` を継続安定させる追加調整
- `next-transport` の `transport_two_process_roundtrip_static_peer` は local harness ではまだ handshake が不安定なため ignore 扱い。fast lane は `FakeTransport` と scenario runner を required に固定している。
- Tauri backend binding と実 GUI smoke の導入
- legacy / next の cutover 条件の明文化
