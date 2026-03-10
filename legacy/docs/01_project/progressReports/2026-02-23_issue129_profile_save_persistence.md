# Issue #129 プロフィール保存永続化修正レポート

作成日: 2026年02月23日

## 概要

- 目的:
  - プロフィール保存でアバター以外（表示名・自己紹介など）が永続化されない問題を解消。
  - フロント API 型と Tauri 側 DTO/返却型の不整合を解消。
  - アバター更新機能は維持したまま、保存失敗の可視性を改善。
  - 保存後に他画面へ即時反映されるようキャッシュ同期を追加。
- 主な変更対象:
  - `kukuri-tauri/src/components/settings/ProfileEditDialog.tsx`
  - `kukuri-tauri/src/components/auth/ProfileSetup.tsx`
  - `kukuri-tauri/src/lib/api/tauri.ts`
  - `kukuri-tauri/src/lib/profile/profileSave.ts`（新規）
  - `kukuri-tauri/src/lib/profile/profileQuerySync.ts`（新規）
  - `kukuri-tauri/src-tauri/src/presentation/dto/user_dto.rs`
  - `kukuri-tauri/src-tauri/src/presentation/commands/user_commands.rs`
  - `kukuri-tauri/src-tauri/src/application/services/user_service.rs`

## 実装詳細

1. ローカルプロフィール保存経路の追加

- `TauriApi.updateUserProfile` を追加し、`update_user_profile` コマンドへ `npub/name/displayName/about/picture/nip05` を送信。
- `ProfileEditDialog` と `ProfileSetup` は保存時に以下を順序実行:
  - プライバシー設定更新（必要時）
  - ローカルプロフィール更新（新規）
  - Nostr metadata 更新
  - アバター同期
- これにより、アバター以外の項目もローカル DB に保存される。

2. 型/バリデーション整合

- Tauri 側に `UpdateUserProfileRequest` を追加し、空/長さチェックを実装。
- `update_user_profile` コマンドで `UserMetadata` に変換して `UserService::update_profile` を呼び出す。
- `UserService::update_profile` は対象ユーザー未存在時に `NotFound` を返すよう変更。
- `get_user` / `get_user_by_pubkey` の返却型を `Option<UserProfileDto>` に統一し、フロント側 DTO 期待値に合わせた。

3. 保存エラー可視化の改善

- 工程別ラベル（avatar/privacy/local profile/nostr metadata/avatar sync/nostr init）を導入。
- 複数失敗時は重複除去した詳細メッセージをトースト表示。
- 既存 `errorHandler` ログ経路は維持。

4. 画面間反映（クロススクリーン）改善

- `syncProfileQueryCaches` を追加し、保存成功後に `userProfile` キャッシュ更新と関連クエリ invalidate を実施。
- `ProfileEditDialog` / `ProfileSetup` 両方で同ヘルパーを使用して反映タイミングを統一。

## テスト

- `ProfileEditDialog.test.tsx`
  - `updateUserProfile` 呼び出し検証を追加。
  - アバター更新時に `picture` が保存 payload に反映されることを検証。
  - 失敗時トーストが工程別詳細を含むことを検証。
  - `userProfile` クエリキャッシュ反映を検証。
- `ProfileSetup.test.tsx`
  - 保存成功・表示名フォールバック・アバター保存それぞれで `updateUserProfile` 呼び出しを検証。
  - 失敗時詳細トースト表示を検証。
- `profileTestUtils.tsx`
  - `TauriApi.updateUserProfile` の mock を追加。

## 実行コマンド

- `cd kukuri-tauri/src-tauri && cargo fmt`
- `cd kukuri-tauri/src-tauri && cargo test`
- `bash ./scripts/test-docker.sh ts --scenario profile-avatar-sync`
- `bash ./scripts/test-docker.sh lint --no-build`
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
