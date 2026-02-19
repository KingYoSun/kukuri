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

# Community node tests/build (default on all OS: containerized)
docker compose -f docker-compose.test.yml up -d community-node-postgres
docker compose -f docker-compose.test.yml build test-runner
docker run --rm --network kukuri_community-node-network \
  -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn \
  -v "$(git rev-parse --show-toplevel):/workspace" \
  -w /workspace/kukuri-community-node \
  kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"
```

> Community node tests are containerized by default on every OS.  
> **Windows**: For Tauri tests too, use `./scripts/test-docker.ps1 <suite>` instead of host-direct `pnpm test` / `cargo test`.

## Monorepo layout

```
.
├── kukuri-tauri/           # Desktop app (React + Tauri)
├── kukuri-community-node/  # Community node services
├── docs/                   # Design, implementation, and runbooks
├── scripts/                # Dev/test automation
└── docker/                 # Docker assets
```

| Name | Path | What it does | How to run / test |
| --- | --- | --- | --- |
| Desktop app | `kukuri-tauri/` | Tauri + React client | `cd kukuri-tauri && pnpm tauri dev` / `pnpm test` |
| Rust core (Tauri) | `kukuri-tauri/src-tauri/` | Rust backend, migrations, SQLite | `cd kukuri-tauri/src-tauri && cargo test` |
| Community node | `kukuri-community-node/` | Community node services + `cn` CLI (`p2p bootstrap/relay`) | Containerized default: `docker compose -f docker-compose.test.yml up -d community-node-postgres` + `docker compose -f docker-compose.test.yml build test-runner` + `docker run ... kukuri-test-runner ... cargo test --workspace --all-features` |

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

# Community node / cn-cli
docker compose -f docker-compose.test.yml up -d community-node-postgres
docker compose -f docker-compose.test.yml build test-runner
docker run --rm --network kukuri_community-node-network \
  -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn \
  -v "$(git rev-parse --show-toplevel):/workspace" \
  -w /workspace/kukuri-community-node \
  kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"
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

CI is defined in `./.github/workflows/test.yml` and includes Docker test suites, native Linux tests (Rust + TS), community node tests, format checks, Windows build checks, and desktop E2E scenarios. Community node local verification should follow the same container-first command path as above.

## Contributing & Support

- Open an issue to discuss changes before large work.
- Keep changes scoped and aligned with the existing docs under `./docs/`.
- Run the relevant tests for the area you touched (see Quickstart and Development workflow).

## License

MIT. See [LICENSE](./LICENSE).
