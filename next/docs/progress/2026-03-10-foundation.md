# 2026年03月10日 foundation

## 実装済み
- root Cargo workspace と `cargo xtask` alias
- `next-core`, `next-store`, `next-transport`, `next-app-api`, `next-harness`
- `next-desktop-runtime` を追加し、desktop 用 command surface を pure Rust で検証できるようにした
- `next/apps/desktop` の Linux-first shell
- `next/apps/desktop/src-tauri` の thin wrapper を追加し、`invoke` 経由で `create_post/list_timeline/list_thread/get_sync_status/import_peer_ticket` を呼べる形にした
- remote-sync 向けに `KUKURI_NEXT_BIND_ADDR` / `KUKURI_NEXT_ADVERTISE_HOST` / `KUKURI_NEXT_INSTANCE` を導入し、loopback 固定を外せるようにした
- `next-fast.yml`, `next-nightly.yml`
- `next-transport` は公式 `iroh-gossip` example / docs に寄せて `receiver.joined()` ベースの join gating を導入
- `next-transport` に low-level baseline test を追加し、wrapper 依存の問題と `iroh-gossip` 本体の問題を分離できるようにした

## 検証済み
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`
- Linux 実機で `pnpm tauri dev` を使った `post -> restart -> persist`
- Linux 実機 2 台で固定 port / 相互 ticket import により `connected: yes, peers: 1` の双方向収束を確認
- Linux 実機 2 台で投稿伝播と、peer 終了後に polling で `connected: no, peers: 0` へ戻ることを確認
- `cargo check --manifest-path next/apps/desktop/src-tauri/Cargo.toml`

## 既知の制約
- `next-transport` は ticket からの direct connect と 2-process gossip roundtrip を required に昇格済み
- Tauri backend binding は導入済み。次の実機確認は `reply/thread 伝播` と `複数 topic 同時購読`。
