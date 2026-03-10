# Nightly CI Community Node E2E 失敗対応
日付: 2026年02月01日

## 概要
- Nightly の Community Node E2E が WebDriver 接続失敗（127.0.0.1:5061）になる問題を安定化。
- tauri-driver/proxy の起動判定と WDIO 起動前チェックを追加。

## 変更点
- startDriver: Linux で driver が起動済み・proxy 未起動の場合に proxy を起動して待機。
- proxy 起動済み判定を listening で判定し、非 listening の場合は close → 再起動。
- driver / proxy の error をログ化。
- wdio.desktop: driver ポートが応答しない場合に startDriver を呼ぶ。

## 検証
- `gh run view 21546372969 --log-failed`
- `gh act --workflows .github/workflows/nightly.yml --job community-node-e2e`
  - E2E 実行は通過（Spec Files 7/7 passed）。
  - Upload Artifact は act の制約で `ACTIONS_RUNTIME_TOKEN` が無く失敗。
  - ログ: `tmp/logs/community-node-e2e/20260131-182559.log`
  - レポート: `test-results/community-node-e2e/20260131-182559/`
- `gh act --workflows .github/workflows/test.yml --job format-check --env NPM_CONFIG_PREFIX=/tmp/npm-global`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux --env NPM_CONFIG_PREFIX=/tmp/npm-global --env CARGO_TEST_THREADS=1 --env RUST_TEST_THREADS=1`

## 備考
- ローカル環境で `kukuri-p2p-bootstrap` が残っていると docker compose が name conflict を起こすため、必要に応じて `docker rm -f kukuri-p2p-bootstrap` を実行。