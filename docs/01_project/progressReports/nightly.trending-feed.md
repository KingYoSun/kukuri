# Nightly `trending-feed` Runbook
最終更新日: 2025年11月17日

## 概要
- Nightly Frontend Unit Tests 後段の Docker シナリオとして `/trending` `/following` ルート専用の Vitest セットと `trending_metrics_job` 監視を実行する。
- `nightly.yml` では実行前に `tmp/logs/trending-feed/`・`tmp/logs/trending_metrics_job_stage4_*.log`・`test-results/trending-feed/` をクリーンアップし、1 ジョブ = 1 セットの artefact だけを `nightly.trending-feed-*` 命名で公開する。

## GitHub Actions ジョブ
- ファイル: `.github/workflows/nightly.yml` / job: `trending-feed`（cron `0 15 * * *`）。
- シナリオ実行: `./scripts/test-docker.sh ts --scenario trending-feed`（`prometheus-trending` サービスを自動起動し、Vitest JSON / Stage4 ログ / Prometheus スナップショット / `p2p_metrics_export --job trending` を取得）。
- artefact:
  - `nightly.trending-feed-logs`: `tmp/logs/trending-feed/latest.log`・`tmp/logs/trending-feed/<timestamp>.log`・`tmp/logs/trending_metrics_job_stage4_<timestamp>.log` と `test-results/trending-feed/prometheus/*.log`
  - `nightly.trending-feed-reports`: `test-results/trending-feed/reports/*.json`
  - `nightly.trending-feed-metrics`: `test-results/trending-feed/metrics/*.json`（`DATABASE_URL` 未設定時はスキップし、CI では `if-no-files-found: warn`）

## ローカル再現

### Corepack / pnpm 初期化（Windowsホスト）
- `scripts/test-docker.ps1 all` / `ts -Scenario trending-feed` は実行前に Corepack shim と `node_modules/.modules.yaml` を検証するため、Windows では先に `cmd.exe` から pnpm を初期化しておく。
- 実行例:
  ```powershell
  cmd.exe /c "corepack enable pnpm"
  cmd.exe /c "corepack pnpm install --frozen-lockfile"
  ```
- macOS / Linux では同じコマンドをターミナルから実行する（`corepack enable pnpm && corepack pnpm install --frozen-lockfile`）。Runbook ではテストログと一緒に「Corepack + pnpm 初期化済み」の記録を残し、再現時に同じ環境を保証する。

### Docker スクリプト
- Bash: `./scripts/test-docker.sh ts --scenario trending-feed [--fixture tests/fixtures/trending/<file>.json]`
- PowerShell: `.\scripts\test-docker.ps1 ts -Scenario trending-feed`
- 成果物はローカルでも同じディレクトリ (`tmp/logs/trending-feed/`, `test-results/trending-feed/{reports,prometheus,metrics}/`) に保存される。

### `gh act` で Nightly を再現
- コマンド:  
  `gh act --bind --workflows .github/workflows/nightly.yml --job trending-feed --container-options "--privileged" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest`
- `--bind` でホストの作業ツリーをジョブコンテナへマウントしないと `docker compose` がホスト側 Volume を解決できず artefact が欠落する。  
  `--container-options "--privileged"` によって Docker Sock (`/var/run/docker.sock` / Windows: `//./pipe/docker_engine`) へアクセスできるようにし、permission denied を回避する。
- `act` では `actions/upload-artifact` が利用できないため、`env.ACT == 'true'` の場合はアップロードをスキップし、代わりに `ls tmp/logs/trending-feed` / `find test-results/trending-feed` を出力する `List trending feed outputs (act)` ステップで生成物を確認する。
- 実行ログは `tmp/gh-act-trending-feed.log` / `tmp/gh-act-trending-feed-after.log` に保存。

## トリアージで確認するログ
- `tmp/logs/trending-feed/latest.log` : 各 Vitest ターゲットの結果と `VITE_TRENDING_FIXTURE_PATH`。
- `tmp/logs/trending_metrics_job_stage4_<timestamp>.log` : Prometheus エンドポイント / curl 出力 / Compose ログ。
- `test-results/trending-feed/reports/<timestamp>-*.json` : `vitest --reporter=json` で生成された詳細結果。
- `test-results/trending-feed/metrics/<timestamp>-trending-metrics.json` : `p2p_metrics_export --job trending --pretty` のスナップショット（DB 未作成時は出力無し）。

## 実施コマンド
- `gh act --bind --workflows .github/workflows/nightly.yml --job trending-feed --container-options "--privileged" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest *> tmp/gh-act-trending-feed.log`
- `./scripts/test-docker.ps1 ts -Scenario trending-feed`
