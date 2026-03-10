# Issue #142 Phase2 timeline aggregation rebuild 実装レポート

作成日: 2026年02月23日

## 概要

- 目的:
  - Topic タイムラインを thread 単位で再構築し、親投稿・先頭返信・返信数・最終アクティビティを集約して表示する。
  - バックエンド API とフロント表示を一体で更新し、投稿変化時の再描画整合も確保する。
- 結果:
  - `get_topic_timeline` 集約 API を実装し、Topic 画面を Timeline card 表示へ移行。
  - 返信関連メタデータと最終活動時刻の表示、関連テスト、キャッシュ無効化連携まで完了。

## 実装内容

1. バックエンド集約 API

- `PostRepository` に `get_topic_timeline(topic_id, limit)` を追加。
- SQL 集約クエリ `SELECT_TOPIC_TIMELINE_SUMMARIES` を追加し、thread ごとに以下を返却:
  - `root_event_id`
  - `first_reply_event_id`
  - `reply_count`
  - `last_activity_at`
- `PostService::get_topic_timeline` を追加し、時刻をフロント向けに整形して返却。
- DTO / Handler / Command / `lib.rs` command 登録まで接続し、Tauri command として公開。

2. フロントの Timeline card 化

- `TauriApi.getTopicTimeline` と型定義を追加。
- `useTopicTimeline(topicId)` を追加し、`parentPost` / `firstReply` / `replyCount` / `lastActivityAt` にマッピング。
- `TimelineThreadCard` コンポーネントを新規実装:
  - 親投稿カード
  - 先頭返信プレビュー
  - 返信件数
  - 最終アクティビティ表示
- `topics.$topicId` を `useTopicTimeline + TimelineThreadCard` へ置換。

3. キャッシュ整合とローカライズ

- 投稿追加/更新/削除、P2P/Nostr イベント受信時に `['topicTimeline', topicId]` を invalidate する経路を追加。
- QueryClient の重要クエリに `topicTimeline` を追加。
- i18n キーを追加:
  - `topics.timelineReplies`
  - `topics.timelineLastActivity`
  - `topics.timelineFirstReply`
  - 対象: `ja` / `en` / `zh-CN`

4. テスト

- Rust:
  - `get_topic_timeline_returns_parent_first_reply_counts_and_last_activity` を追加。
- Frontend:
  - `usePosts.test.tsx` に `useTopicTimeline` テスト追加。
  - `TimelineThreadCard.test.tsx` を新規追加（表示とフォールバック検証）。

## テスト実行

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
