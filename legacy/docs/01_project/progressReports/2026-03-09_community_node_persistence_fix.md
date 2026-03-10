# Linux クライアントの account / Community Node 永続化修正レポート

作成日: 2026年03月09日
最終更新日: 2026年03月09日

## 1. 概要

- Linux headless / keyring unavailable 環境で、account 情報と Community Node 認証状態が再起動後に失われ、常に onboarding へ戻る問題を修正した。
- Rust の file-backed fallback 実装に加えて、`browser.reloadSession()` を使った live-path E2E を追加し、アプリ再起動後も home / settings / token status が復元されることを確認した。

## 2. 実施内容

### 2.1 secure storage fallback の永続化

- `kukuri-tauri/src-tauri/src/infrastructure/storage/secure_storage.rs`
  - keyring が利用できない場合の fallback を in-memory only から file-backed へ変更。
  - 保存先は `dirs::data_local_dir()/kukuri/secure_storage_fallback.json`。
  - テスト用に `KUKURI_SECURE_STORAGE_FALLBACK_DIR` override を利用可能にした。
- 追加 test
  - `fallback_persistence_restores_current_account_after_in_memory_reset`

### 2.2 live-path E2E の追加

- `kukuri-tauri/tests/e2e/specs/community-node.persistence.spec.ts`
  - account 作成
  - Community Node 認証
  - `browser.reloadSession()`
  - `waitForAppReady()` / `waitForHome()`
  - settings で node entry と `data-has-token=true` を確認
  の流れを固定した。

## 3. 検証

- `./scripts/test-docker.ps1 rust`: pass
- `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.persistence.spec.ts ./scripts/test-docker.ps1 e2e-community-node`: pass
  - ログ: `tmp/logs/community-node-e2e/20260309-002559.log`

## 4. タスク管理反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 「Linux ローカル永続化不具合」を削除。
- `docs/01_project/activeContext/tasks/completed/2026-03-09.md`
  - 完了内容と検証結果を追記。

## 5. 残課題

- Windows reload crash (`iroh-quinn ... PoisonError`)
- Admin UI connected users / health の live-path 確認
