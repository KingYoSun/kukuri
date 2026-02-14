日本語 | [English](./README.md)

# kukuri

Nostr と iroh-gossip、BitTorrent Mainline DHT を基盤にした、完全分散型のトピック指向ソーシャルアプリです。

## これは何か

kukuri は Tauri デスクトップアプリと周辺サービスで構成され、中央サーバーに依存しないトピック共有体験を提供します。イベント配信は iroh-gossip、ピア発見は DHT、データモデルは Nostr 互換イベントを採用しています。

## クイックスタート

### 前提

- Node.js 20+
- pnpm（Corepack 経由）
- Rust toolchain
- Docker（Docker テストランナーと community node 用）

### インストール

```bash
chmod +x scripts/install-dev-tools.sh
./scripts/install-dev-tools.sh

corepack enable pnpm
cd kukuri-tauri
corepack pnpm install --frozen-lockfile
```

### 起動（デスクトップアプリ）

```bash
cd kukuri-tauri
corepack pnpm tauri dev
```

### テスト / Lint（最小）

```bash
# Docker でフルテスト
./scripts/test-docker.sh all

# フロントエンドテスト（Linux/macOS/WSL2）
cd kukuri-tauri
pnpm test

# Rust テスト（Linux/macOS/WSL2）
cd kukuri-tauri/src-tauri
cargo test

# Community node テスト/ビルド（全OS既定: コンテナ実行）
docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch
docker run --rm --network kukuri_community-node-network \
  -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn \
  -e MEILI_URL=http://community-node-meilisearch:7700 \
  -e MEILI_MASTER_KEY=change-me \
  -v "$(git rev-parse --show-toplevel):/workspace" \
  -w /workspace/kukuri-community-node \
  kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"
```

> Community node テストは全OSでコンテナ実行を既定とします。  
> **Windows**: Tauri 側も含め、`pnpm test` / `cargo test` のホスト直実行は避け、`./scripts/test-docker.ps1 <suite>` を使用してください。

## モノレポ構成

```
.
├── kukuri-tauri/           # デスクトップアプリ（React + Tauri）
├── kukuri-community-node/  # community node サービス
├── docs/                   # 設計/実装/運用ドキュメント
├── scripts/                # 開発/テスト自動化
└── docker/                 # Docker 関連
```

| 名称 | パス | 役割 | 起動 / テスト |
| --- | --- | --- | --- |
| デスクトップアプリ | `kukuri-tauri/` | Tauri + React クライアント | `cd kukuri-tauri && pnpm tauri dev` / `pnpm test` |
| Rust コア（Tauri） | `kukuri-tauri/src-tauri/` | Rust バックエンド + SQLite | `cd kukuri-tauri/src-tauri && cargo test` |
| Community node | `kukuri-community-node/` | Community node サービス群 + `cn` CLI（`p2p bootstrap/relay`） | コンテナ既定: `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch` + `docker run ... kukuri-test-runner ... cargo test --workspace --all-features` |

## 開発フロー

### 主要コマンド

```bash
# デスクトップアプリ
cd kukuri-tauri
pnpm tauri dev
pnpm tauri build
pnpm lint
pnpm format
pnpm type-check
pnpm test

# Rust（Tauri）
cd kukuri-tauri/src-tauri
cargo test
cargo clippy -D warnings

# Community node / cn-cli
docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch
docker run --rm --network kukuri_community-node-network \
  -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn \
  -e MEILI_URL=http://community-node-meilisearch:7700 \
  -e MEILI_MASTER_KEY=change-me \
  -v "$(git rev-parse --show-toplevel):/workspace" \
  -w /workspace/kukuri-community-node \
  kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"
```

### Docker テストランナー

```bash
# Docker で全テスト
./scripts/test-docker.sh all

# Windows（PowerShell）
./scripts/test-docker.ps1 all
```

## 設定

### 環境変数ファイル

- `./.env.example`（ブートストラップ/リレーのシークレット等）
- `./kukuri-community-node/.env.example`（community node サービス設定）

#### Community node のセットアップ

```bash
cd kukuri-community-node
cp .env.example .env
```

#### 手動検証用 P2P ブートストラップ（任意）

```bash
docker compose -f docker-compose.test.yml up -d p2p-bootstrap
# ...検証後...
docker compose -f docker-compose.test.yml down --remove-orphans
```

## アーキテクチャ（概要）

```mermaid
graph TD
  A[Client: Tauri App] --> B[Discovery: BitTorrent DHT]
  A --> C[P2P Network: iroh-gossip]
  C --> D[Marketplace: Search/Suggestion Nodes]
```

## CI

CI は `./.github/workflows/test.yml` で定義されており、Docker テスト、Linux ネイティブテスト（Rust + TS）、community node テスト、フォーマットチェック、Windows ビルドチェック、デスクトップ E2E を含みます。community node のローカル検証も上記のコンテナ経路を既定にしてください。

## 貢献・サポート

- 大きな変更の前に Issue で相談してください。
- 変更内容は `./docs/` の方針に合わせて整理してください。
- 変更対象に応じたテストを実行してください（Quickstart / 開発フロー参照）。

## ライセンス

MIT。詳細は [LICENSE](./LICENSE) を参照してください。
