# 直結 multi-peer の listener 重複登録修正レポート

作成日: 2026年03月09日
最終更新日: 2026年03月09日

## 1. 概要

- direct multi-peer live-path で受信した topic post が `postStore.postsByTopic` に重複登録され、実機相当 diagnostics では同一 event id が複数回積まれていた。
- 原因は `useP2P()` を複数コンポーネントで使う構成に対して、`useP2PEventListener` が mount ごとに Tauri listener を登録していたことだった。
- listener の共有化と store 側の idempotent 化を行い、focused unit regression と multi-peer E2E の両方で再発を止めた。

## 2. 実施内容

### 2.1 実機寄りの回帰を先に固定

- `kukuri-tauri/src/tests/unit/hooks/useP2PEventListener.test.tsx`
  - `listen` mock を複数 handler 対応へ拡張し、複数 mount 時に同一 topic post が重複登録される failing regression を追加した。
- `kukuri-tauri/tests/e2e/specs/p2p.direct-peer.regression.spec.ts`
  - single-arrival 時点の `postStore` / `p2pStore` diagnostics を比較し、direct multi-peer live-path の崩れを即座に検出できるようにした。

### 2.2 listener 多重登録の修正

- `kukuri-tauri/src/hooks/useP2PEventListener.ts`
  - module-scope の ref count を導入し、`useP2PEventListener` が複数 mount されても Tauri event subscription は 1 組だけ維持するよう変更した。
  - 最後の mount が unmount されたタイミングでのみ cleanup するようにし、Strict Mode 相当の再 mount にも耐える形へ整理した。

### 2.3 store 側の防御追加

- `kukuri-tauri/src/stores/postStore.ts`
  - `addPost` / `setPosts` の `postsByTopic` 更新を unique append に変更し、同一 `post.id` が topic 配列へ二重に入らないようにした。
- `kukuri-tauri/src/tests/unit/stores/postStore.test.ts`
  - 同じ `post.id` を再投入しても topic 内で重複しない unit test を追加した。

### 2.4 E2E 判定の実機整合性を改善

- `kukuri-tauri/tests/e2e/specs/p2p.direct-peer.regression.spec.ts`
  - real device では initial fetch 済みの post store と P2P message store の baseline が一致しない場合があるため、single-arrival 判定を absolute count 比較から baseline 差分比較へ変更した。
  - 既存投稿を含む状態でも、新規 event の重複だけを正しく検出できるようにした。

## 3. 検証結果

- `docker compose -f docker-compose.test.yml run --rm ts-test bash /app/scripts/docker/run-vitest-target.sh src/tests/unit/hooks/useP2PEventListener.test.tsx /app/test-results/useP2PEventListener.unit.json`
  - 結果: pass
- `docker compose -f docker-compose.test.yml run --rm ts-test bash /app/scripts/docker/run-vitest-target.sh src/tests/unit/stores/postStore.test.ts /app/test-results/postStore.unit.json`
  - 結果: pass
- `E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts ./scripts/test-docker.ps1 e2e-multi-peer`
  - 結果: pass
  - ログ: `tmp/logs/multi-peer-e2e/20260309-053647.log`
  - diagnostics: `test-results/multi-peer-e2e/direct-peer-regression.json`
  - 確認事項: `baselinePostStoreSnapshot.count=2 -> postStoreAfterSingleArrival.count=3`、`baselineP2PSnapshot.count=1 -> p2pSnapshotAfterPropagation.count=2` となり、増分は双方とも 1 で一致した。

## 4. タスク管理反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 直結 multi-peer の realtime 差分更新メモを更新し、listener 重複登録修正と baseline 差分比較 E2E の検証結果を追記した。

## 5. 残課題

- IPv6-only 条件で stale render が再発するかを切り分ける専用 live-path E2E は未整備。
- direct multi-peer / IPv6 条件での reply 投稿失敗 / thread 導線差分は引き続き未整理。
