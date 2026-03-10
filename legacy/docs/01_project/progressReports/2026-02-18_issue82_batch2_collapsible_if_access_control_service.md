# Issue #82 batch 2: `clippy::collapsible_if`（`access_control_service.rs`）

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/82

## 実施概要

Issue #82 の batch2 として、`kukuri-tauri/src-tauri/src/application/services/access_control_service.rs` に残っていた `clippy::collapsible_if` を局所解消した。対象は join request 検証と invite 検証の条件分岐で、挙動を変えない最小差分の構文変換に限定した。

## 変更内容

- 対象ファイル: `kukuri-tauri/src-tauri/src/application/services/access_control_service.rs`
- 変更方針:
  - 入れ子 `if` のみを `if let ... && ...` へ統合
  - 条件式の意味、エラー文字列、戻り値、分岐順序は不変更
  - DTO / presentation 層は未変更
- 解消件数: 16件

## 検証結果

- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo fmt --all`
  - pass
- `cd kukuri-tauri/src-tauri && set -o pipefail; CARGO_HOME=/tmp/cargo-home cargo clippy --all-targets --all-features -- -W clippy::collapsible_if 2>&1 | tee /tmp/issue82_batch2_clippy.log`
  - pass
  - `clippy::collapsible_if` の lib 警告が 66 -> 50 に減少（-16）
  - `/tmp/issue82_batch2_clippy.log` 上で `access_control_service.rs` の `collapsible_if` 指摘は 0 件
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test`
  - pass（unit / integration / contract を含む既存スイート）

## 補足

- 本セッションではユーザー明示指示に従い `gh act` は実行していない。
- 残件（lib 観測値）: 50件。次バッチでは別のまとまり（サービス層または DTO 層）で段階的に解消する。
