# Issue #145 Phase5 realtime timeline mode 実装レポート

作成日: 2026年02月23日

## 概要

- 目的:
  - topic timeline の更新モードに `standard` / `realtime` を導入する。
  - realtime モードで差分反映をバッチ適用し、頻繁な全体 refetch を避ける。
  - 切断やオフライン時は `realtime` から `standard` へ安全にフォールバックする。
- 結果:
  - UI トグル・永続化・差分イベント経路・バッチ適用 hook を実装。
  - mode 切替時の invalidate/dispatch 分岐を Nostr/P2P 双方に適用。
  - realtime 利用不能時の自動フォールバックを追加し、継続利用時の壊れた表示を防止。

## 実装内容

1. 更新モードの状態管理と UI

- `uiStore` に `timelineUpdateMode`（`standard` / `realtime`）と setter を追加。
- persist 設定に `timelineUpdateMode` を追加して再起動後もモードを保持。
- `TimelineModeToggle` コンポーネントを新規追加し、Topic 画面に統合。

2. realtime 差分イベント基盤

- `timelineRealtimeEvents.ts` を新規追加し、timeline 差分イベント型と dispatch 関数を定義。
- `useNostrEvents` / `useP2PEventListener` を更新:
  - `standard` モード: 既存どおり query invalidate。
  - `realtime` モード: `topic timeline delta` を dispatch。

3. バッチ適用とフォールバック

- `useRealtimeTimeline` を新規追加。
- 差分受信時は 750ms のバッファでまとめ、同一 thread を集約して一括で `topicTimeline` キャッシュへ反映。
- 適用不能差分（thread 未存在など）はバッチ単位で refetch を 1 回だけ実行。
- `offline` イベントや切断時コールバックで `standard` へ自動フォールバック。

4. ルート統合と query 挙動

- `topics.$topicId.tsx` で `timelineUpdateMode` と `useRealtimeTimeline` を統合。
- `useTopicTimeline(topicId, mode)` を拡張し、`realtime` では polling を無効化（`standard` のみ 30 秒 polling）。

5. テスト

- 追加:
  - `useRealtimeTimeline.test.tsx`（バッチ適用、refetch 単発化、offline フォールバック）
- 更新:
  - `topics.$topicId.test.tsx`（トグル操作と mode setter 呼び出し）
  - `uiStore.test.ts`（初期値と setter）

## 検証

- `docker compose -f docker-compose.test.yml run --rm --no-deps ts-test pnpm test -- --run --reporter=dot --silent`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
