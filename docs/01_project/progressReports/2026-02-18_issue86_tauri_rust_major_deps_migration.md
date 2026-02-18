# Issue #86: `kukuri-tauri` Rust major 依存（`reqwest` / `rand` / `rand_core` / `bincode`）移行

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/86

## 実施概要

`kukuri-tauri/src-tauri` の主要 Rust 依存について、`Cargo.toml` を起点にメジャー更新を実施した。`reqwest` / `rand` / `rand_core` は最新互換メジャーへ更新し、rand 0.10 系 API 変更に合わせて実装コードとテストコードを修正した。

`bincode` は crates.io 上の最新 `3.0.0` を検証したが、upstream 側で `compile_error!("https://xkcd.com/2347/")` が埋め込まれており実用利用不可能なため、`2.0.1` を維持した。

## 変更ファイル

- `kukuri-tauri/src-tauri/Cargo.toml`
- `kukuri-tauri/src-tauri/Cargo.lock`
- `kukuri-tauri/src-tauri/src/application/services/access_control_service.rs`
- `kukuri-tauri/src-tauri/src/application/services/profile_avatar_service.rs`
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/bootstrap.rs`
- `kukuri-tauri/src-tauri/src/infrastructure/p2p/dht_bootstrap.rs`
- `kukuri-tauri/src-tauri/src/domain/p2p/message.rs`
- `kukuri-tauri/src-tauri/tests/p2p_mainline_smoke.rs`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/activeContext/tasks/completed/2026-02-18.md`

## 依存更新結果

- `reqwest`: `0.12.x -> 0.13.2`（`default-features = false`, features: `json`, `query`, `rustls`）
- `rand`: `0.9.x -> 0.10.0`
- `rand_core`: `0.9.x -> 0.10.0`
- `bincode`: `2.0.1` 維持（`3.0.0` は upstream compile error により採用不可）

## API 差分対応

- `OsRng` / `TryRngCore` ベースの乱数取得を `SysRng` / `TryRng` へ置換。
- `rand::Rng` 拡張を利用していた箇所を `rand::RngExt` へ置換。
- `secp256k1` の RNG 系統差異回避のため、関連テストを `secp256k1::rand::rng()` 利用に変更。

## 実行コマンドと結果

- `cd kukuri-tauri/src-tauri && cargo search reqwest --limit 1`
- `cd kukuri-tauri/src-tauri && cargo search rand --limit 1`
- `cd kukuri-tauri/src-tauri && cargo search rand_core --limit 1`
- `cd kukuri-tauri/src-tauri && cargo search bincode --limit 1`
  - 最新公開版を確認（`reqwest 0.13.2`, `rand 0.10.0`, `rand_core 0.10.0`, `bincode 3.0.0`）
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo update`
  - lock 同期（必要な feature 調整込み）
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo update -p bincode@3.0.0 --precise 2.0.1`
  - `bincode` を利用可能版へ固定
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo check`
  - pass
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test`
  - pass
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo clippy -- -D warnings`
  - fail（既存の `clippy::collapsible_if` 指摘。今回変更差分外）
- `cd kukuri-tauri/src-tauri && RUSTUP_HOME=/tmp/rustup-home CARGO_HOME=/tmp/cargo-home cargo +1.86.0 check`
  - fail（`iroh` 系依存が Rust 1.88+ を要求）
- `cd kukuri && DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
  - pass
- `cd kukuri && DOCKER_CONFIG=/tmp/docker-config docker compose -f docker-compose.test.yml build test-runner`
  - pass
- `cd kukuri && DOCKER_CONFIG=/tmp/docker-config docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
  - pass
- `DOCKER_CONFIG=/tmp/docker-config NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check --action-cache-path /tmp/act --cache-server-path /tmp/actcache`
  - pass
- `DOCKER_CONFIG=/tmp/docker-config NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux --action-cache-path /tmp/act --cache-server-path /tmp/actcache`
  - pass
- `DOCKER_CONFIG=/tmp/docker-config NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests --action-cache-path /tmp/act --cache-server-path /tmp/actcache`
  - pass

## 課題とエスカレーション方針

`bincode 3.0.0` は upstream が意図的に compile error を埋め込んでおり、現時点で移行不能。Issue #86 へ次の選択肢を提示してエスカレーションする。

1. `bincode 2.0.1` を維持し、`3.x` は採用可能版公開後に再評価。
2. `bincode` 代替シリアライザ導入可否を別 Issue で調査。
3. upstream の方針変更または後継 crate 公開待ち。
