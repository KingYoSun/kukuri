# Docker 複数 Peer クライアント実装レポート

作成日: 2026年02月28日
最終更新日: 2026年02月28日

## 1. 概要

- ローカル複数 Peer 接続の再現検証のため、Docker コンテナベースの headless Peer クライアントを実装した。
- `kukuri-tauri` の既存 P2P 実装と Desktop E2E 導線を再利用し、自動検証と手動操作の両方を同一基盤で運用可能にした。

## 2. 実装内容

### 2.1 Headless Peer 実装

- 追加: `kukuri-tauri/src-tauri/src/bin/p2p_peer_harness.rs`
- 実装モード:
  - `listener`
  - `publisher`
  - `echo`
- 出力:
  - peer ごとの実行ログ
  - peer ごとの JSON サマリ（`KUKURI_PEER_SUMMARY_PATH`）

### 2.2 Docker・スクリプト導線

- 更新: `docker-compose.test.yml`
  - `peer-client-1` / `peer-client-2` / `peer-client-3` を追加
  - `test-runner` へ multi-peer 用環境変数を追加
- 追加:
  - `scripts/docker/run-multi-peer-e2e.sh`
  - `scripts/docker/run-multi-peer-manual.sh`
- 更新:
  - `scripts/test-docker.ps1`
  - `scripts/test-docker.sh`
  - `scripts/docker/run-smoke-tests.sh`
  - `scripts/docker/run-desktop-e2e.sh`
  - `Dockerfile.test`

### 2.3 Desktop E2E 連携

- 追加: `kukuri-tauri/tests/e2e/specs/community-node.multi-peer.spec.ts`
- 更新: `kukuri-tauri/tests/e2e/wdio.desktop.ts`
  - `E2E_SPEC_PATTERN` で spec 切替可能に変更
  - `SCENARIO=multi-peer-e2e` 時のみ `KUKURI_BOOTSTRAP_PEERS` を有効化

### 2.4 CI 連携

- 更新: `.github/workflows/test.yml`
  - `desktop-e2e-multi-peer` ジョブを追加
  - `push-heavy-checks` に `desktop-e2e-multi-peer` を追加

### 2.5 手動運用ドキュメント

- 更新: `docs/03_implementation/p2p_mainline_runbook.md`
  - 「Docker 複数 Peer クライアント運用」章を追加
  - `multi-peer-up|status|down` の手順とログ採取先を明記
- 更新: `docs/01_project/activeContext/docker_multi_peer_client_plan.md`
  - ステータスを `Implemented` に変更
  - 実装結果（Phase 1〜5）を追記

## 3. 検証結果

### 3.1 実装機能検証

- `./scripts/test-docker.ps1 e2e-multi-peer`
  - 結果: pass
  - ログ: `tmp/logs/multi-peer-e2e/20260228-083948.log`
  - 成果物: `test-results/multi-peer-e2e/20260228-083948`

### 3.2 `gh act` 必須ジョブ

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
  - 結果: fail（既存の Prettier 差分）
  - 差分ファイル:
    - `src/components/p2p/BootstrapConfigPanel.tsx`
    - `src/components/RelayStatus.tsx`
    - `src/components/settings/CommunityNodePanel.tsx`
    - `src/hooks/useP2PEventListener.ts`
    - `src/lib/networkRefreshEvent.ts`
    - `src/tests/unit/components/RelayStatus.test.tsx`
    - `src/tests/unit/components/settings/CommunityNodePanel.test.tsx`
  - ログ: `tmp/act-logs/20260228-180158-format-check.log`

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - 結果: pass
  - ログ: `tmp/act-logs/20260228-180325-native-test-linux.log`

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`
  - 結果: fail（既知事象）
  - 失敗理由: `gh act` コンテナ内の `/workspace/kukuri-community-node` で `Cargo.toml` を解決できず `error: could not find Cargo.toml` が発生
  - ログ: `tmp/act-logs/20260228-181038-community-node-tests.log`

## 4. 補足

- `format-check` 失敗は今回実装差分とは独立した既存フォーマット差分に起因。
- `community-node-tests` 失敗は `gh act` ローカル実行時の既知マウント差異によるもので、Docker 依存サービス起動自体は正常に完了している。
