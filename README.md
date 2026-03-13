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
├── next/              # active Linux-first implementation
├── next/docs/         # current truth: adr / runbook / progress
├── legacy/            # archived pre-cutover assets and docs history
└── .github/workflows/ # next-fast.yml / next-nightly.yml
```

## Rules

- `next/` is the active implementation surface.
- `legacy/` is reference-only.
- Linux is the only required target during MVP.
- Windows, DHT discovery, and community-node integration are deferred.
- `AGENTS.md -> next/docs/*` is sufficient for new contributors.

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
