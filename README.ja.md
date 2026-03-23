日本語 | [English](./README.md)

# kukuri

Linux-first で再構築中の、Nostr ベースの topic-first social app です。

## 現在の入口

新規開発は root workspace を対象にします。

```bash
cargo xtask doctor
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke

cd apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

## 現在の構成

```text
.
├── apps/              # 現行の desktop app
├── crates/            # 現行の Rust 実装
├── docs/              # 現在の真実: adr / runbook / progress
├── harness/           # scenario 定義
├── legacy/            # cutover 前の資産と履歴アーカイブ
└── .github/workflows/ # kukuri-fast.yml / kukuri-nightly.yml
```

## ルール

- root workspace が実装対象です。
- `legacy/` は参照専用です。
- 現在の desktop target は Linux / Windows です。
- 現在の network scope には static-peer、seeded DHT、community-node connectivity/auth が含まれます。
- UI/UX 作業は `docs/adr/0014-uiux-dev-flow.md` と `docs/DESIGN.md` に従います。
- 新規参加者は `AGENTS.md -> docs/*` だけで着手できます。

## ドキュメント

- overview: `docs/README.md`
- decision: `docs/adr/0001-linux-first-mvp.md`
- ui/ux flow: `docs/adr/0014-uiux-dev-flow.md`
- design rules: `docs/DESIGN.md`
- runbook: `docs/runbooks/dev.md`
- progress: `docs/progress/2026-03-10-foundation.md`

## 検証済み entrypoint

- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`

## ライセンス

MIT
