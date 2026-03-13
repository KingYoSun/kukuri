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
├── next/              # 現行の Linux-first 実装
├── next/docs/         # 現在の真実: adr / runbook / progress
├── legacy/            # cutover 前の資産と履歴アーカイブ
└── .github/workflows/ # next-fast.yml / next-nightly.yml
```

## ルール

- `next/` が実装対象です。
- `legacy/` は参照専用です。
- MVP 中は Linux だけを required target にします。
- Windows、DHT、community-node 連携は後続フェーズです。
- 新規参加者は `AGENTS.md -> next/docs/*` だけで着手できます。

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
