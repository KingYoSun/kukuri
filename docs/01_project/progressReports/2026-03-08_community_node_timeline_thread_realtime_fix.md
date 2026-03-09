# Community Node 実機UXのリアルタイム timeline / thread 導線修正レポート

作成日: 2026年03月08日
最終更新日: 2026年03月08日

## 1. 概要

- Community Node 経由で受信した投稿が「トピックメッシュ」には表示される一方、リアルタイムモードのタイムラインには即時反映されない不具合を修正した。
- reply の伝播後に thread UI が更新されず、`「スレッドを開く」` が設計どおりの右ペイン導線ではなく thread 一覧へ戻ってしまう不具合を修正した。
- いずれも shortcut ではなく live-path E2E を追加し、external peer からの root post / reply を persistent peer harness 経由で注入して再現・回帰検証できるようにした。

## 2. 実施内容

### 2.1 リアルタイム timeline 更新の修正

- `kukuri-tauri/src/hooks/useRealtimeTimeline.ts`
  - P2P 受信 payload に `eventId` を保持し、thread 情報が欠けている場合は store fallback で `threadUuid` / `threadRootEventId` / `threadParentEventId` を補完するよう変更。
- `kukuri-tauri/src/hooks/useP2PEventListener.ts`
  - incoming P2P `Post` に `eventId: message.id` を保存し、reply event の thread 情報を store から補完できるよう変更。
- `kukuri-tauri/src/lib/posts/postMapper.ts`
  - realtime 差分から取り込んだ投稿が thread UI と同一の識別子で扱われるよう整合を取った。

### 2.2 thread 右ペイン導線の修正

- `kukuri-tauri/src/routes/topics.$topicId.threads.tsx`
  - `/topics/$topicId/threads/$threadUuid` 遷移時は thread 一覧ではなく `Outlet` を返し、右ペイン詳細が維持されるよう修正。
- `kukuri-tauri/src/tests/unit/routes/topics.$topicId.threads.test.tsx`
  - 詳細 route で `Outlet` が描画される unit test を追加。

### 2.3 live-path E2E の整備

- `kukuri-tauri/src-tauri/src/bin/p2p_peer_harness.rs`
  - command file を監視し、実行中 peer に publish 指示を送れるよう拡張。
- `docker-compose.test.yml`
  - `peer-client-*` に command directory を注入し、persistent peer harness を外部制御できるようにした。
- `kukuri-tauri/tests/e2e/helpers/peerHarness.ts`
  - command enqueue / result wait helper を追加。
- `kukuri-tauri/tests/e2e/specs/community-node.timeline-thread-realtime.spec.ts`
  - external peer の root post が reload なしで realtime timeline に反映されることを検証。
- `kukuri-tauri/tests/e2e/specs/community-node.thread-preview-replies.spec.ts`
  - external peer reply の受信後に timeline / thread preview / thread detail が更新され、`「スレッドを開く」` が thread 一覧へ戻らないことを検証。

## 3. 検証結果

- `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.thread-preview-replies.spec.ts ./scripts/test-docker.ps1 e2e-community-node`: pass
  - ログ: `tmp/logs/community-node-e2e/20260308-183533.log`
- `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.timeline-thread-realtime.spec.ts ./scripts/test-docker.ps1 e2e-community-node`: pass
  - ログ: `tmp/logs/community-node-e2e/20260308-183827.log`

## 4. タスク管理反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 完了済みの「リアルタイムモードのタイムライン即時反映」「スレッド右ペイン導線」課題を削除し、進捗メモへ検証結果を追記。
- `docs/01_project/activeContext/tasks/completed/2026-03-08.md`
  - 完了内容と検証結果を追記。

## 5. 残課題

- profile 伝播と false failure toast
- Community Node 設定/認証/role 変更時の false failure toast
- Windows reload crash (`iroh-quinn ... PoisonError`)
- Linux の account / Community Node 設定永続化
- relay URL fallback 汚染と iroh path warning
- Admin UI の connected users / health 不整合
