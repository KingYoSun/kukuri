# Issue #82 batch 1: `clippy::collapsible_if`（`src/shared/config.rs`）

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/82

## 実施概要

`clippy::collapsible_if` 73件のうち、安全に挙動維持しやすい設定読み込み系の局所範囲として、`kukuri-tauri/src-tauri/src/shared/config.rs` の7件を第1バッチで解消した。

## 変更内容

- 対象ファイル: `kukuri-tauri/src-tauri/src/shared/config.rs`
- 変更方針:
  - 入れ子 `if` の構造のみを変換し、分岐条件・代入先・エラー文字列は不変更
  - `if let ... { if let ... { ... } }` を `if let ... && let ... { ... }` へ変換
  - `if let Some(port) ... { if port == 0 { ... } }` を `if let Some(port) = ... && port == 0 { ... }` へ変換
- 解消件数: 7件

## 検証結果

- `cd kukuri-tauri/src-tauri && set -o pipefail; CARGO_HOME=/tmp/cargo-home cargo clippy --all-targets --all-features -- -W clippy::collapsible_if 2>&1 | tee /tmp/kukuri-tauri-clippy-issue82-batch1.log`
  - pass
  - `clippy::collapsible_if` の lib 警告が 73 -> 66 に減少（`config.rs` 分を解消）
- `cd kukuri-tauri/src-tauri && set -o pipefail; CARGO_HOME=/tmp/cargo-home cargo test 2>&1 | tee /tmp/kukuri-tauri-test-issue82-batch1.log`
  - pass（230 unit + integration/contract/perf skip を含む既存スイート）
- `cd kukuri-tauri/src-tauri && set -o pipefail; CARGO_HOME=/tmp/cargo-home cargo clippy --locked --workspace --all-features -- -D warnings -A dead_code -A unused_variables -A clippy::collapsible_if 2>&1 | tee /tmp/kukuri-tauri-clippy-dwarn-issue82-batch1.log`
  - pass

## セッション必須 gh act

- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job format-check 2>&1 | tee tmp/logs/gh-act-format-check-issue82-batch1.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job native-test-linux 2>&1 | tee tmp/logs/gh-act-native-test-linux-issue82-batch1.log`（pass）
- `NPM_CONFIG_PREFIX=/tmp/npm-global XDG_CACHE_HOME=/tmp/xdg-cache DOCKER_CONFIG=/tmp/docker-config ACT_CACHE_DIR=/tmp/act-cache gh act --workflows .github/workflows/test.yml --job community-node-tests 2>&1 | tee tmp/logs/gh-act-community-node-tests-issue82-batch1.log`（pass）

## 残タスク（Issue #82 継続分）

- `clippy::collapsible_if` 残件（今回計測時点）
  - lib: 66件
  - 主要残ファイル: `src/application/services/access_control_service.rs`、`src/presentation/dto/offline.rs` ほか
- 次バッチでは、同様に挙動リスクが低い DTO/バリデーション層から段階的に縮減する。
