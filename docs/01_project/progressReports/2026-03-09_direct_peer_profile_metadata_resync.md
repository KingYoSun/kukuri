# 直結 multi-peer のプロフィール表示名解決レポート

作成日: 2026年03月09日
最終更新日: 2026年03月09日

## 1. 概要

- `p2p.direct-peer.regression.spec.ts` で、投稿本文は realtime 反映される一方、author 表示だけが短縮 pubkey のまま残る不具合を再現した。
- 原因は UI 側の表示処理ではなく、direct peer 接続前に peer harness が送っていた profile metadata(kind=0) を listener 側が取り逃がしていたことだった。
- peer 接続後に metadata を同じ multi-peer 経路で再送するよう修正し、実機寄りの live-path E2E で reload なしの表示名解決を固定した。

## 2. 実施内容

### 2.1 failing E2E で実際の症状を再現

- `E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts ./scripts/test-docker.ps1 e2e-multi-peer` を実行し、`profileResolved === true` の期待が失敗する状態を再現した。
- diagnostics と peer summary を確認し、本文投稿は受信しているが metadata は listener 側に届いていないことを確認した。

### 2.2 peer join 後の metadata 再送を追加

- `kukuri-tauri/src-tauri/src/bin/p2p_peer_harness.rs`
  - `PeerJoined` を受けたとき、`publish_metadata` が有効な peer harness は profile metadata(kind=0) を再送するよう変更した。
  - これにより、接続確立後の direct peer 経路でも profile metadata が確実に流れるようにした。

### 2.3 E2E を実経路に合わせて整理

- `kukuri-tauri/tests/e2e/specs/p2p.direct-peer.regression.spec.ts`
  - test 側で別経路の補助 publish を入れず、peer join 後に自然に流れてくる metadata を待つ形へ整理した。
  - `profileResolvedBeforeMetadataSync` / `profileResolvedWithoutReload` の diagnostics を残し、接続直後は未解決でも metadata 到着後に reload なしで表示名へ切り替わることを確認できるようにした。
- `kukuri-tauri/tests/e2e/helpers/peerHarness.ts`
  - 既存の publish command helper を共通化し、command enqueue の重複を整理した。

## 3. 検証結果

- `E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts ./scripts/test-docker.ps1 e2e-multi-peer`
  - 結果: pass
  - ログ: `tmp/logs/multi-peer-e2e/20260309-030407.log`
- `E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts` + `E2E_DIRECT_PEER_CONNECT_MODE=direct` で `./scripts/test-docker.ps1 e2e-multi-peer`
  - 結果: pass
  - ログ: `tmp/logs/multi-peer-e2e/20260309-030607.log`
- `./scripts/test-docker.ps1 rust`
  - 結果: pass
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - 結果: pass
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - 結果: pass
  - 補足: 初回実行で `p2p.direct-peer.regression.spec.ts` の未使用 `bodyText` を ESLint が検出したため、その 1 行を削除して再実行した。
- `gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - 結果: pass

## 4. タスク管理反映

- `docs/01_project/activeContext/tasks/status/in_progress.md`
  - 完了済みの「相手プロフィール表示名解決」を削除し、残課題を realtime / reply-thread 差分のみに整理した。
- `docs/01_project/activeContext/tasks/completed/2026-03-09.md`
  - 完了内容と検証結果を追記した。
