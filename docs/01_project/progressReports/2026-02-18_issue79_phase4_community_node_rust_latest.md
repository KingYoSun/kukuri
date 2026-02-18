# Issue #79 Phase 4: `kukuri-community-node` Rust dependencies latest

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/79

## 実施概要

`kukuri-community-node` の Rust 依存を最新化した。更新途中で Rust 1.86（CI / Docker test-runner）と MSRV が衝突したため、エスカレーション条件に基づき選択肢を整理し、Phase 4 は「現行 Rust 1.86 制約下での最大更新」で着地した。

## 対象マニフェスト（列挙）

- `kukuri-community-node/Cargo.toml`
- `kukuri-community-node/Cargo.lock`
- `kukuri-community-node/crates/cn-admin-api/Cargo.toml`
- `kukuri-community-node/crates/cn-bootstrap/Cargo.toml`
- `kukuri-community-node/crates/cn-cli/Cargo.toml`
- `kukuri-community-node/crates/cn-core/Cargo.toml`
- `kukuri-community-node/crates/cn-index/Cargo.toml`
- `kukuri-community-node/crates/cn-kip-types/Cargo.toml`
- `kukuri-community-node/crates/cn-moderation/Cargo.toml`
- `kukuri-community-node/crates/cn-relay/Cargo.toml`
- `kukuri-community-node/crates/cn-trust/Cargo.toml`
- `kukuri-community-node/crates/cn-user-api/Cargo.toml`

## マネージャーエスカレーション（実施）

依存更新直後に以下を確認:

- `iroh 0.96.x` / `iroh-gossip 0.96.x` が `rustc >= 1.89` 必須
- `test-runner` は `rust:1.86-bookworm` 固定

選択肢:

1. テスト基盤（Dockerfile.test / CI）を `rustc >= 1.89` に引き上げて true latest を採用
2. 基盤は 1.86 のまま、1.86 で成立する最新版に制約して更新

Phase 4 ではスコープ逸脱を避けるため **Option 2** を採用。

## 変更内容

### 依存制約（`Cargo.toml`）

- 更新:
  - `anyhow 1.0.86 -> 1.0.101`
  - `chrono 0.4.38 -> 0.4.43`
  - `clap 4.5.4 -> 4.5.59`
  - `futures-util 0.3.31 -> 0.3.32`
  - `reqwest 0.12.8 -> 0.13.2`（feature を `json,rustls,query` へ）
  - `regex 1.10.5 -> 1.12.3`
  - `serde 1.0.203 -> 1.0.228`
  - `serde_json 1.0.117 -> 1.0.149`
  - `sqlx 0.7.4 -> 0.8.6`
  - `tokio 1.38.0 -> 1.49.0`
  - `tower 0.5.2 -> 0.5.3`
  - `tracing 0.1.40 -> 0.1.44`
  - `tracing-subscriber 0.3.18 -> 0.3.22`
  - `utoipa 4.2.3 -> 5.4.0`
  - `uuid 1.8.0 -> 1.21.0`
- 互換性調整:
  - `jsonwebtoken` は `default-features = false` を追加（`simple_asn1` 由来の MSRV 乖離を回避）
- 据え置き（1.86 制約）:
  - `iroh = 0.95.1`
  - `iroh-gossip = 0.95.0`
  - `rand_core = 0.6.4`

### ロック更新（`Cargo.lock`）

- `cargo update` 実行後、Rust 1.86 互換維持のため以下を lock 固定:
  - `home 0.5.11`
  - `time 0.3.45`
  - `time-core 0.1.7`
  - `time-macros 0.2.25`

### 追従修正

- `utoipa 5` の OpenAPI 出力に合わせ、契約テスト期待値を `3.1.0` へ更新。
  - `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - `kukuri-community-node/crates/cn-user-api/src/openapi_contract_tests.rs`

## 検証結果

### 直接検証（Community Node）

- `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`（pass）
- `docker compose -f docker-compose.test.yml build test-runner`（pass）
- `docker run --rm --network kukuri_community-node-network ... cargo test --workspace --all-features; cargo build --release -p cn-cli`（pass）

### セッション必須 `gh act`

- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 初回: `~/.cache/act` permission error
  - `XDG_CACHE_HOME=/tmp/xdg-cache` 指定で再実行し pass
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（pass）
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`（pass）

## carry-over

- true latest（`iroh 0.96.x` 系）適用には CI / Docker test-runner の Rust 1.89+ 化が前提。
- 本フェーズでは依存更新のみを対象とし、基盤 Rust 引き上げは follow-up 判断事項として残す。
