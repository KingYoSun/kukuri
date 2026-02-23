# Issue #143 Phase3 thread UI routes / forum view 実装レポート

作成日: 2026年02月23日

## 概要

- 目的:
  - Topic 配下に thread 専用ルートを追加し、一覧と詳細を分離する。
  - 詳細画面を forum-style 表示にして、階層返信と expand/collapse を提供する。
- 結果:
  - `/topics/$topicId/threads` と `/topics/$topicId/threads/$threadUuid` を追加し、一覧・詳細導線を実装。
  - `thread_parent_event_id` ベースのツリー描画、返信の階層表示、折りたたみ制御を実装。
  - 取得 hook、キャッシュ無効化、i18n、テスト、CI 相当検証を完了。

## 実装内容

1. ルート追加と導線更新

- `topics.$topicId.threads.tsx` を追加し、topic の thread 一覧画面を実装。
- `topics.$topicId.threads.$threadUuid.tsx` を追加し、thread 詳細画面を実装。
- `topics.$topicId.tsx` で `/threads` 配下を `Outlet` へ委譲し、`open-topic-threads-button` を追加。
- `TimelineThreadCard` に thread 詳細へのリンク導線（`topicId` 付き）を追加。
- `routeTree.gen.ts` を更新し、新規 2 ルートを反映。

2. thread 取得 hook / キャッシュ整合

- `usePosts.ts` に `useTopicThreads(topicId)` と `useThreadPosts(topicId, threadUuid)` を追加。
- 投稿更新経路で `topicThreads` / `threadPosts` を invalidate するよう以下を更新:
  - `src/lib/posts/cacheUtils.ts`
  - `src/stores/postStore.ts`
  - `src/hooks/useNostrEvents.ts`
  - `src/hooks/useP2PEventListener.ts`

3. forum-style thread detail

- `ForumThreadView` を新規追加し、thread 詳細の root/replies セクションを描画。
- `forumThreadTree.ts` を新規追加し、`threadParentEventId` から木構造を構築する `buildThreadTree` を実装。
- 子孫返信数を使ったノード単位 expand/collapse を実装。
- 親不在返信は `detached roots` として分離表示。

4. i18n とテスト

- i18n キーを `ja/en/zh-CN` に追加（threads 一覧、root/replies、展開/折りたたみ等）。
- 追加/更新テスト:
  - `ForumThreadView.test.tsx`
  - `topics.$topicId.threads.test.tsx`
  - `topics.$topicId.threads.$threadUuid.test.tsx`
  - `usePosts.test.tsx`（新 hooks）
  - `TimelineThreadCard.test.tsx`（thread 遷移導線）

## 検証

- `bash ./scripts/test-docker.sh ts`
- `bash ./scripts/test-docker.sh lint --no-build`
- `cd kukuri-tauri/src-tauri && cargo test`
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker compose -f docker-compose.test.yml build test-runner`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
