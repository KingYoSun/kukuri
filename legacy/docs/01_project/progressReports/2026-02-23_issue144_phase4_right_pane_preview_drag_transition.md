# Issue #144 Phase4 右ペイン preview / drag 遷移 実装レポート

作成日: 2026年02月23日

## 概要

- 目的:
  - タイムラインの親投稿クリックで thread preview を右ペイン表示する。
  - preview から左ドラッグ閾値で full thread ルートへ遷移できるようにする。
  - ドラッグ操作が難しい環境向けに、非ドラッグ代替操作を提供する。
- 結果:
  - `TimelineThreadCard` 親投稿クリックで `ThreadPreviewPane` を表示する導線を追加。
  - 右ペインで左ドラッグ 120px 超過時に `/topics/$topicId/threads/$threadUuid` へ遷移。
  - 「全画面で開く」ボタンを常設し、a11y fallback を満たした。

## 実装内容

1. タイムラインカードから preview 起動

- `TimelineThreadCard` に `onParentPostClick` を追加。
- 親投稿コンテナへ `role=button` / `tabIndex` / `Enter` / `Space` 対応を追加。
- 既存の interactive 要素クリック時は preview 起動を抑止。

2. 右ペイン preview 実装

- `ThreadPreviewPane` を新規追加。
- `useThreadPosts(topicId, threadUuid)` で preview 内容を表示。
- 左ドラッグ閾値（`120px`）到達で `onOpenFullThread` を実行。
- fallback として `thread-preview-open-full` ボタンを常設。
- close 操作（`thread-preview-close`）を追加。

3. ルート統合

- `topics.$topicId.tsx` に `previewThreadUuid` state を追加。
- タイムライン + preview の 2 カラムレイアウトを実装。
- preview の full 操作で `/topics/$topicId/threads/$threadUuid` へ遷移するよう接続。
- テストのため `TopicPage` を export 化。

4. i18n とテスト

- `ja/en/zh-CN` に preview 操作文言を追加。
- 追加/更新テスト:
  - `ThreadPreviewPane.test.tsx`
  - `topics.$topicId.test.tsx`
  - `TimelineThreadCard.test.tsx`

## 検証

- `docker compose --project-name kukuri_tests -f docker-compose.test.yml run --rm lint-check`
- `bash ./scripts/test-docker.sh ts --no-build`
- `bash ./scripts/test-docker.sh rust --no-build`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
