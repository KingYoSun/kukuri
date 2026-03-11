# Development Runbook

## 初回セットアップ
```bash
npx pnpm@10.16.1 install --dir next/apps/desktop
cargo xtask doctor
```

## 日常コマンド
```bash
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
```

## frontend だけ確認する場合
```bash
cd next/apps/desktop
npx pnpm@10.16.1 dev
npx pnpm@10.16.1 test
npx pnpm@10.16.1 tauri:dev
```

## 次の手動確認
1. `npx pnpm@10.16.1 tauri:dev` を 2 instance 起動する。
2. 両方の `Your Ticket` を相互に `Peer Ticket` へ貼って import する。
3. 片方で post し、もう片方の timeline に反映されることを確認する。
4. 片方を再起動しても timeline が維持されることを確認する。

## 現在の注意点
- `next-transport` の `transport_static_peer_can_connect_endpoint` は required。
- `next-transport` の `transport_two_process_roundtrip_static_peer` は required に戻した。
- deterministic な required lane は `FakeTransport` と `next-harness` が担う。
- Tauri wrapper の単体 compile は `cargo check --manifest-path next/apps/desktop/src-tauri/Cargo.toml` で確認する。
