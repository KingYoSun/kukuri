# Issue #79 Phase 3: `kukuri-tauri/src-tauri` Rust dependencies latest（Option 1）

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/79

## 実施概要

Manager 判断（Option 1）に従い、Phase 3 は依存更新を優先して出荷し、`cargo clippy -D warnings` で発生した広範囲の既存警告修正は follow-up Issue に分離した。

## 変更内容

- `cd kukuri-tauri/src-tauri && cargo update`
  - `Cargo.lock` の依存を最新へ更新（69 package update）
- `cd kukuri-tauri/src-tauri && cargo update -p time --precise 0.3.45`
  - CI の Rust 1.86 と互換な `time` 系へ調整

## 追補（Manager 指示: Cargo.toml 更新必須）

Manager 指示「`Cargo.lock` 単体更新ではなく `Cargo.toml` も更新」を反映し、`kukuri-tauri/src-tauri/Cargo.toml` の直接依存制約を Rust 1.86 互換範囲で引き上げた。

- 更新した直接依存（Cargo.toml）:
  - `anyhow 1.0.100 -> 1.0.101`
  - `blake3 1.8.2 -> 1.8.3`
  - `bytes 1.11.0 -> 1.11.1`
  - `chrono 0.4.42 -> 0.4.43`
  - `futures 0.3.31 -> 0.3.32`
  - `lru 0.16.2 -> 0.16.3`
  - `serde_json 1.0.147 -> 1.0.149`
  - `tauri-plugin-fs 2.4.4 -> 2.4.5`
  - `tauri-plugin-opener 2.5.2 -> 2.5.3`
  - `thiserror 2.0.17 -> 2.0.18`
  - `tokio 1.48.0 -> 1.49.0`
  - `uuid 1.19.0 -> 1.21.0`
  - `criterion 0.8.1 -> 0.8.2`
  - `tempfile 3.24.0 -> 3.25.0`
- `cargo update` 再実行時に `time 0.3.47` 系へ上がると `rustc 1.88+` が必要になるため、`cargo update -p time --precise 0.3.45` で Rust 1.86 互換へ固定。
- 今回据え置いた主な依存:
  - `iroh` / `iroh-gossip`: 最新 `0.96.x` は `rust_version = 1.89` のため CI 制約（1.86）に抵触。
  - `bincode` / `rand` / `rand_core` / `reqwest`: いずれもメジャー更新（`3.0` / `0.10` / `0.13`）で API 影響が大きく、依存更新 PR からは分離（risk-later）。

## 検証結果

### 直接検証

- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test`（pass）
- `cd kukuri-tauri/src-tauri && set -o pipefail; CARGO_HOME=/tmp/cargo-home cargo clippy -- -D warnings 2>&1 | tee /tmp/kukuri-tauri-clippy-issue79-phase3.log`
  - fail: `clippy::collapsible_if` 73件（既存実装の広範囲に分布）
- `cd kukuri-tauri/src-tauri && RUSTUP_HOME=/tmp/rustup-home CARGO_HOME=/tmp/cargo-home cargo +1.86.0 check`（pass）

### セッション必須 gh act

- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue79-phase3.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue79-phase3.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue79-phase3.log`（pass）

## 分離した follow-up

- Issue: https://github.com/KingYoSun/kukuri/issues/82
- タイトル: `[tech-debt] kukuri-tauri clippy::collapsible_if 73件の段階的解消`
- 分離理由:
  - 依存更新PRに同梱すると修正範囲が広く、意図しない挙動変更リスクが高い
  - Option 1 の方針に合わせ、更新出荷を先行し clippy 是正を独立トラックで管理する
