# 2026-02-21 Issue #111 NIP-85 trust migration

## 概要

- 目的: legacy trust model（39010/39011）から NIP-85 trust model（30382-30385 / 10040）へ全面移行する。
- スコープ: `kukuri-community-node` / `kukuri-tauri` の trust 型・定数・検証・生成・API/DTO・UI・テスト・関連 docs。

## 実装内容

- `cn-kip-types` / `cn-core` / `cn-trust` / `cn-user-api` で trust event 種別を NIP-85 に置換し、assertion/provider list を扱う経路へ更新。
- Tauri 側 `community_node_handler` / DTO / command / API client / UI を NIP-85 trust provider ベースへ更新。
- legacy `39011` は既存保存値移行のためテスト専用互換レイヤーを保持し、運用経路からは除去。
- Admin Console の trust 設定を `attestation` 表記から `assertion` 表記へ更新（後方互換読取あり）。
- trust 関連テストを更新し、kind 型不一致（`u32` vs `u16`）を修正。

## 検証結果

- Community Node container test: pass
- Tauri Rust test（`bash ./scripts/test-docker.sh rust`）: pass
- Tauri TypeScript test（`bash ./scripts/test-docker.sh ts`）: pass
- lint（`docker compose -f docker-compose.test.yml run --rm lint-check`）: pass
- `gh act`:
  - `format-check`: pass
  - `native-test-linux`: pass
  - `community-node-tests`: pass

## 補足

- `39010/39011` は実装コード上では `legacy trust anchor` 移行テスト（`kind:39011`）のみ残置。
- docs 内の過去ログ/完了タスクには履歴として `39010/39011` 記述が残る。
