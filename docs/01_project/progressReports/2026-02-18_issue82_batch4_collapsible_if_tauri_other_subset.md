# 2026-02-18 Issue #82 batch4 `clippy::collapsible_if`

## 対象
- `kukuri-tauri/src-tauri/src/presentation/dto/offline.rs`
- `kukuri-tauri/src-tauri/src/presentation/dto/post_dto.rs`
- `kukuri-tauri/src-tauri/src/application/shared/mappers/users.rs`
- `kukuri-tauri/src-tauri/src/application/services/access_control_service.rs`
- `kukuri-tauri/src-tauri/src/domain/entities/event_gateway/profile_metadata.rs`

## 変更内容
- `if let Some(x) = ... { if ... { ... }}` の入れ子を
  `if let Some(x) = ... && ... { ... }` へ、
  挙動不変・エラー文言・戻り値を維持して最小差分で置換。
- `#82` の残存件数を `41` から更に減らすための batch4。

## 検証
- `git diff` にて対象5ファイルの差分を確認。
- `cargo` 実行環境が存在しないため、当日環境ではローカル `cargo fmt/clippy/test` を実行できず（`/bin/bash: cargo: command not found`）。
  CI 上で PR の required checks 実行を想定。
