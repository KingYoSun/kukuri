# Issue #82 batch 3: `clippy::collapsible_if`（`presentation/dto` subset）

作成日: 2026年02月18日
Issue: https://github.com/KingYoSun/kukuri/issues/82

## 実施概要

Issue #82 の batch3 として、`kukuri-tauri/src-tauri/src/presentation/dto` のうち `if` 入れ子解消が安全に局所化できる 3 ファイル（`access_control_dto.rs` / `topic_dto.rs` / `user_dto.rs`）を対象に、`clippy::collapsible_if` を最小差分で解消した。

## 変更内容

- 対象ファイル:
  - `kukuri-tauri/src-tauri/src/presentation/dto/access_control_dto.rs`
  - `kukuri-tauri/src-tauri/src/presentation/dto/topic_dto.rs`
  - `kukuri-tauri/src-tauri/src/presentation/dto/user_dto.rs`
- 変更方針:
  - 入れ子 `if` の構造のみを `if let ... && ...` へ変換
  - バリデーション条件、エラーメッセージ、戻り値は不変更
  - DTO 以外の層（service / infrastructure / command）は未変更
- 解消件数: 9件

## 検証結果

- `cd kukuri-tauri/src-tauri && cargo fmt --all`
  - pass
- `cd kukuri-tauri/src-tauri && set -o pipefail; CARGO_HOME=/tmp/cargo-home cargo clippy --all-targets --all-features -- -Aclippy::all -Wclippy::collapsible_if 2>&1 | tee /tmp/issue82_batch3_clippy_after.log`
  - pass
  - `clippy::collapsible_if` 件数: total 53 -> 44（-9）
  - `clippy::collapsible_if` の lib 警告: 50 -> 41（-9）
- `cd kukuri-tauri/src-tauri && CARGO_HOME=/tmp/cargo-home cargo test`
  - pass（unit / integration / contract / smoke を含む既存スイート）

## 補足

- 本セッションではユーザー明示指示に従い `gh act` は実行していない。
- 残件（lib 観測値）: 41件。次バッチでは残存の DTO / service / handler のまとまりを継続解消する。
