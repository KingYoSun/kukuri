English | [日本語](./README.ja.md)

# kukuri

A fully decentralized, topic-first social app built on Nostr, iroh-gossip, and BitTorrent Mainline DHT.

## What it is

kukuri is a Tauri desktop application and supporting services that enable topic-based social sharing without relying on central servers. It uses iroh-gossip for fast event distribution and DHT-based discovery for peer connectivity, with Nostr-compatible events as the data model.

## Quickstart

### Prerequisites

- Node.js 20+
- pnpm (via Corepack)
- Rust toolchain
- Docker (for the Docker test runner and community node)

### Install

```bash
chmod +x scripts/install-dev-tools.sh
./scripts/install-dev-tools.sh

corepack enable pnpm
cd kukuri-tauri
corepack pnpm install --frozen-lockfile
```

### Run (desktop app)

```bash
cd kukuri-tauri
corepack pnpm tauri dev
```

### Test / Lint (short list)

```bash
# Full test suite in Docker
./scripts/test-docker.sh all

# Frontend tests (Linux/macOS/WSL2)
cd kukuri-tauri
pnpm test

# Rust tests (Linux/macOS/WSL2)
cd kukuri-tauri/src-tauri
cargo test
```

> **Windows**: Use `./scripts/test-docker.ps1 <suite>` instead of running `pnpm test` / `cargo test` directly on the host.

## Monorepo layout

```
.
├── kukuri-tauri/           # Desktop app (React + Tauri)
├── kukuri-cli/             # Bootstrap/relay CLI for DHT
├── kukuri-community-node/  # Community node services
├── docs/                   # Design, implementation, and runbooks
├── scripts/                # Dev/test automation
└── docker/                 # Docker assets
```

| Name | Path | What it does | How to run / test |
| --- | --- | --- | --- |
| Desktop app | `kukuri-tauri/` | Tauri + React client | `cd kukuri-tauri && pnpm tauri dev` / `pnpm test` |
| Rust core (Tauri) | `kukuri-tauri/src-tauri/` | Rust backend, migrations, SQLite | `cd kukuri-tauri/src-tauri && cargo test` |
| CLI node | `kukuri-cli/` | DHT bootstrap + relay CLI | `cd kukuri-cli && cargo build --release` / `cargo test` |
| Community node | `kukuri-community-node/` | Minimal community node services | `cd kukuri-community-node && docker compose up -d` / `cargo test --workspace --all-features` |

## Development workflow

### Common commands

```bash
# Desktop app
cd kukuri-tauri
pnpm tauri dev
pnpm tauri build
pnpm lint
pnpm format
pnpm type-check
pnpm test

# Rust (Tauri)
cd kukuri-tauri/src-tauri
cargo test
cargo clippy -D warnings

# CLI
cd kukuri-cli
cargo test
cargo build --release
```

### Docker test runner

```bash
# Run everything in Docker
./scripts/test-docker.sh all

# Windows (PowerShell)
./scripts/test-docker.ps1 all
```

## Configuration

### Environment files

- `./.env.example` (bootstrap/relay secrets and defaults)
- `./kukuri-cli/.env.example` (CLI logging/network defaults)
- `./kukuri-community-node/.env.example` (community node services)

#### Community node setup

```bash
cd kukuri-community-node
cp .env.example .env
```

#### P2P bootstrap for manual validation (optional)

```bash
docker compose -f docker-compose.test.yml up -d p2p-bootstrap
# ...run your checks...
docker compose -f docker-compose.test.yml down --remove-orphans
```

## Architecture (high-level)

```mermaid
graph TD
  A[Client: Tauri App] --> B[Discovery: BitTorrent DHT]
  A --> C[P2P Network: iroh-gossip]
  C --> D[Marketplace: Search/Suggestion Nodes]
```

## CI

CI is defined in `./.github/workflows/test.yml` and includes Docker test suites, native Linux tests (Rust + TS), community node tests, format checks, Windows build checks, and desktop E2E scenarios.

## Contributing & Support

- Open an issue to discuss changes before large work.
- Keep changes scoped and aligned with the existing docs under `./docs/`.
- Run the relevant tests for the area you touched (see Quickstart and Development workflow).

## License

MIT. See [LICENSE](./LICENSE).
