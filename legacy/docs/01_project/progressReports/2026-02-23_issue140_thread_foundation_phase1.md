# Issue #140 Topic timeline/thread rebuild Phase1 実装レポート

作成日: 2026年02月23日

## 概要

- 目的:
  - Topic timeline/thread rebuild の Phase1 として、バックエンド側の thread foundation を実装する。
  - 投稿 DTO と永続化モデルを thread 前提に拡張し、`topic_id + thread_uuid` で取得できる経路を提供する。
  - 既存データへの互換 backfill は行わない。
- 結果:
  - DTO 拡張、`event_threads` モデル導入、`get_thread_posts` 経路追加を完了。
  - フロント呼び出し側も新 DTO に追随し、`thread_uuid` を常に送信するよう統一。

## 実装内容

1. DTO / コマンド拡張

- `CreatePostRequest` に `thread_uuid`（必須）/ `reply_to` を追加。
- `PostResponse` に `thread_namespace` / `thread_uuid` / `thread_root_event_id` / `thread_parent_event_id` を追加。
- `GetThreadPostsRequest` を新規追加。
- Tauri command `get_thread_posts` を追加し、`topic_id + thread_uuid + pagination` を受け付けるようにした。

2. 永続化モデル（event_threads）導入

- migration を追加:
  - `kukuri-tauri/src-tauri/migrations/20260223103000_add_event_threads_table.up.sql`
  - `kukuri-tauri/src-tauri/migrations/20260223103000_add_event_threads_table.down.sql`
- `event_threads` テーブルと必要インデックスを定義。
- 投稿作成時および受信イベント保存時に thread メタを `event_threads` へ保存する経路を実装。

3. Service / Repository 経路追加

- `PostService::create_post` シグネチャを拡張し、`thread_uuid` と `reply_to` を処理。
- 返信投稿は親イベントの thread メタを継承し、root/parent の整合を維持。
- `PostRepository` に以下を追加:
  - `get_posts_by_thread(topic_id, thread_uuid, limit)`
  - `get_event_thread(topic_id, event_id)`
- `PostService::get_thread_posts(topic_id, thread_uuid, limit)` を追加。

4. フロント/同期経路の追随

- `TauriApi.createPost` / 型定義を更新し `thread_uuid` を送信。
- `postStore` / `usePosts` / `syncEngine` / `ReplyForm` / `QuoteForm` / E2E bridge を更新。
- 送信時 `thread_uuid` は「明示指定 > 親の thread > 新規 UUID」で決定。
- 投稿マッパーと store 型に thread メタを追加。

## テスト

- `cd kukuri-tauri/src-tauri && cargo fmt`
- `cd kukuri-tauri/src-tauri && cargo test`
- `bash ./scripts/test-docker.sh ts`
- `bash ./scripts/test-docker.sh lint --no-build`
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
