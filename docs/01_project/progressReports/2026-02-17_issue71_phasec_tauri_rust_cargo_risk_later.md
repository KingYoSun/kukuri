# Issue #71 Phase C: `kukuri-tauri` Rust Cargo risk-later 依存更新

最終更新日: 2026年02月17日

## 概要

Issue #71 の Phase C として、`kukuri-tauri` スコープの Rust Cargo 依存を対象に、risk-later 候補から `tauri` ファミリーの更新のみを最小差分で適用した。

## 対象マニフェスト（網羅）

- `kukuri-tauri/src-tauri/Cargo.toml`
- `kukuri-tauri/src-tauri/Cargo.lock`

## 更新内容

- `tauri`: `2.9.5 -> 2.10.2`
- `tauri-build`: `2.5.3 -> 2.5.5`
- `Cargo.lock`（`tauri` 連動で更新）:
  - `tauri`: `2.9.5 -> 2.10.2`
  - `tauri-build`: `2.5.3 -> 2.5.5`
  - `tauri-codegen`: `2.5.2 -> 2.5.4`
  - `tauri-macros`: `2.5.2 -> 2.5.4`
  - `tauri-plugin`: `2.5.2 -> 2.5.3`
  - `tauri-runtime`: `2.9.2 -> 2.10.0`
  - `tauri-runtime-wry`: `2.9.3 -> 2.10.0`
  - `tauri-utils`: `2.8.1 -> 2.8.2`
  - `wry`: `0.53.5 -> 0.54.2`
  - `webkit2gtk`: `2.0.1 -> 2.0.2`
  - `webkit2gtk-sys`: `2.0.1 -> 2.0.2`
  - `ico`: `0.4.0 -> 0.5.0`
  - `reqwest`: `0.13.2`（追加）
  - `wasm-streams`: `0.5.0`（追加）

変更方針: `cargo update --dry-run` で確認できる広範囲更新（83パッケージ）を避け、`tauri` 直接依存とその必須連動分のみを対象化して回帰範囲を抑制。

## 検証

- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo update -p tauri -p tauri-build -p tauri-codegen -p tauri-macros -p tauri-plugin -p tauri-runtime -p tauri-runtime-wry -p tauri-utils -p wry`（pass）
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test`（pass）
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo clippy --all-targets --all-features -- -D warnings`（fail: 既存 lint）
- `cd /tmp/kukuri-main-issue71/kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo clippy --all-targets --all-features -- -D warnings`（fail: 同一 lint でベースライン再現）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue71-phasec.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue71-phasec.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue71-phasec.log`（pass）

## 変更ファイル

- `kukuri-tauri/src-tauri/Cargo.toml`
- `kukuri-tauri/src-tauri/Cargo.lock`
- `docs/01_project/activeContext/tasks/completed/2026-02-17.md`
- `docs/01_project/activeContext/tasks/status/in_progress.md`
- `docs/01_project/progressReports/2026-02-17_issue71_phasec_tauri_rust_cargo_risk_later.md`

## ログ

- `tmp/logs/issue71-phasec-cargo-update-tauri-family.log`
- `tmp/logs/issue71-phasec-tauri-cargo-test.log`
- `tmp/logs/issue71-phasec-tauri-cargo-clippy.log`
- `tmp/logs/issue71-phasec-tauri-cargo-clippy-baseline-main.log`
- `tmp/logs/gh-act-format-check-issue71-phasec.log`
- `tmp/logs/gh-act-native-test-linux-issue71-phasec.log`
- `tmp/logs/gh-act-community-node-tests-issue71-phasec.log`

## Carry-over

- Phase D（Issue #71 計画）: `kukuri-community-node` Rust Cargo risk-later 依存更新へ移行。
- `kukuri-tauri` Rust 側の未適用候補（`cargo update --dry-run` で確認される `async-executor` / `clap` / `criterion` / `uuid` ほかの広範囲更新）は、影響範囲が大きいため本フェーズでは適用せず、次段で段階適用する。
