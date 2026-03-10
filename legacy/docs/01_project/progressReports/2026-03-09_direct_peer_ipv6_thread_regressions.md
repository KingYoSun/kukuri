# direct peer relay-only / IPv6 回帰固定レポート

作成日: 2026年03月09日
最終更新日: 2026年03月09日

## 1. 概要

- direct multi-peer の relay-only 条件で未整理だった 2 点を、実機相当の live-path E2E として固定した。
  - propagated post が reload なしに描画されるか
  - external reply が thread preview / full thread detail まで届くか
- どちらも `peer-client-2` の relay hint を使って topic join / direct connect し、peer harness command で event を流す構成に揃えた。

## 2. 追加した E2E

### 2.1 stale render 再確認

- 追加: `kukuri-tauri/tests/e2e/specs/p2p.direct-peer.ipv6-thread-regression.spec.ts`
- シナリオ:
  - `SCENARIO=multi-peer-e2e`
  - `KUKURI_IROH_TRANSPORT_PROFILE=relay-only`
  - `peer-client-2` の address snapshot から relay hint を選択
  - local app は realtime mode で topic を開いたまま、peer harness 側の propagated post を待つ
- 検証内容:
  - relay hint が実際に選ばれたこと
  - P2P snapshot が進行したこと
  - `document.body` と timeline snapshot が reload なしに更新されたこと

### 2.2 reply/thread 導線再確認

- 同 spec 内に `updates thread preview and full thread detail with propagated replies under relay-only direct-peer conditions` を追加した。
- シナリオ:
  - local root post を UI から作成
  - post store から persisted `rootEventId` を解決
  - `peer-client-2` に command file を発行し、`reply_to=rootEventId` 付き reply を publish
  - timeline first reply / thread preview pane / `/topics/$topicId/threads/$threadUuid` の full thread detail を順に確認
- 実機との差分を減らすため、Community Node mock や store seed ではなく、既存 multi-peer harness の live-path をそのまま使っている。

## 3. 生成 artefact

- `test-results/multi-peer-e2e/direct-peer-ipv6-stale-render.json`
  - relay hint 選択結果、bootstrap snapshot、timeline/post store 状態を保存
- `test-results/multi-peer-e2e/direct-peer-ipv6-thread-preview-replies.json`
  - root event 解決結果、reply publish summary、preview/detail 遷移結果を保存
- ログ: `tmp/logs/multi-peer-e2e/20260309-073757.log`
- report dir: `test-results/multi-peer-e2e/20260309-073757`

## 4. 検証結果

- `E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.ipv6-thread-regression.spec.ts ./scripts/test-docker.ps1 e2e-multi-peer`
  - 結果: pass
  - 内訳:
    - `renders propagated posts without reload under relay-only direct-peer conditions`
    - `updates thread preview and full thread detail with propagated replies under relay-only direct-peer conditions`

## 5. タスク反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - direct multi-peer / IPv6 条件の stale render / reply-thread 導線タスクを削除
- `docs/01_project/activeContext/tasks/completed/2026-03-09.md`
  - 完了内容を追記
