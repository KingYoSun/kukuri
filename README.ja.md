日本語 | [English](./README.md)

# kukuri

Linux-first で再構築中の、Nostr ベースの topic-first social app です。

## 現在の入口

新規開発は `next/` を対象にします。

```bash
cargo xtask doctor
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke

cd next/apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

## 現在の構成

```text
.
├── next/                 # 新しい Linux-first MVP
├── next/docs/            # 現行ドキュメント
├── legacy/               # 旧 docs / scripts / docker / workflow / AGENTS
├── kukuri-tauri/         # pre-cutover の旧 app tree（参照用）
├── kukuri-community-node/# pre-cutover の旧 service tree（参照用）
└── .github/workflows/    # next-fast.yml / next-nightly.yml
```

## ルール

- `next/` が実装対象です。
- `legacy/` は参照専用です。
- MVP 中は Linux だけを required target にします。
- Windows、DHT、community-node 連携は後続フェーズです。

## ドキュメント

- overview: `next/docs/README.md`
- decision: `next/docs/adr/0001-linux-first-mvp.md`
- runbook: `next/docs/runbooks/dev.md`
- progress: `next/docs/progress/2026-03-10-foundation.md`

## 検証済み entrypoint

- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`

## ライセンス

MIT
