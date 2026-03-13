English | [日本語](./README.ja.md)

# kukuri

A Linux-first rebuild of a Nostr-based topic-first social app.

## Current Entry Point

New work targets the root workspace.

```bash
cargo xtask doctor
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke

cd apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

## Current Layout

```text
.
├── apps/              # current desktop app
├── crates/            # current Rust implementation
├── docs/              # current truth: adr / runbook / progress
├── harness/           # scenario specs
├── legacy/            # archived pre-cutover assets and docs history
└── .github/workflows/ # kukuri-fast.yml / kukuri-nightly.yml
```

## Rules

- root workspace is the active implementation surface.
- `legacy/` is reference-only.
- Linux is the only required target during MVP.
- Windows, DHT discovery, and community-node integration are deferred.
- `AGENTS.md -> docs/*` is sufficient for new contributors.

## Docs

- overview: `docs/README.md`
- decision: `docs/adr/0001-linux-first-mvp.md`
- runbook: `docs/runbooks/dev.md`
- progress: `docs/progress/2026-03-10-foundation.md`

## Verified Entrypoints

- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`

## License

MIT
