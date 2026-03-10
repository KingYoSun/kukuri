# Issue #167 Topic Timeline/Thread 画面導線修正レポート

作成日: 2026年02月25日

## 概要

- 症状:
  - Phase 2/3 で実装済みの Topic Timeline (`/topics/$topicId`) と Thread (`/topics/$topicId/threads`, `/topics/$topicId/threads/$threadUuid`) 画面が、Tauri クライアント操作上ほぼ到達不能だった。
- 影響:
  - サイドバーの参加トピックから遷移すると常にホーム (`/`) が開き、Topic route 群が表示されない。

## 根本原因

- `kukuri-tauri/src/components/layout/Sidebar.tsx` の `handleTopicClick` が `setCurrentTopic(topic)` 後に `navigate({ to: '/' })` を実行していた。
- そのためトピック選択時に TanStack Router の `/topics/$topicId` 系 route へ遷移せず、Timeline/Thread UI が見えない状態になっていた。

## 実装内容

1. サイドバー遷移先の修正

- `handleTopicClick` を次の遷移に変更:
  - `navigate({ to: '/topics/$topicId', params: { topicId } })`

2. 回帰防止テスト更新

- `kukuri-tauri/src/tests/unit/components/layout/Sidebar.test.tsx` の「トピックをクリックするとナビゲーションと選択状態が更新される」ケースを更新。
- 期待値を `/` から `/topics/$topicId` + `params` に変更。

## 検証結果

- Docker `ts-test` コンテナで Sidebar テストと route テスト一式を実行し pass。
- `gh act` 必須 3 ジョブ（`format-check` / `native-test-linux` / `community-node-tests`）を実行しすべて pass。
