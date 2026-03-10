English | [日本語](./README.ja.md)

# kukuri

A Linux-first rebuild of a Nostr-based topic-first social app.

## Current Entry Point

New work targets `next/`.

```bash
cargo xtask doctor
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke

cd next/apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

## Current Layout

```text
.
├── next/                  # new Linux-first MVP
├── next/docs/             # current docs
├── legacy/                # old docs / scripts / docker / workflows / AGENTS
├── kukuri-tauri/          # pre-cutover legacy app tree
├── kukuri-community-node/ # pre-cutover legacy service tree
└── .github/workflows/     # next-fast.yml / next-nightly.yml
```

## Rules

- `next/` is the active implementation surface.
- `legacy/` is reference-only.
- Linux is the only required target during MVP.
- Windows, DHT discovery, and community-node integration are deferred.

## Docs

- overview: `next/docs/README.md`
- decision: `next/docs/adr/0001-linux-first-mvp.md`
- runbook: `next/docs/runbooks/dev.md`
- progress: `next/docs/progress/2026-03-10-foundation.md`

## Verified Entrypoints

- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`

## License

MIT
