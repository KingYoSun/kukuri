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
```

## 現在の注意点
- `next-transport` の `transport_static_peer_can_connect_endpoint` は required。
- `next-transport` の `transport_two_process_roundtrip_static_peer` は required に戻した。
- deterministic な required lane は `FakeTransport` と `next-harness` が担う。
