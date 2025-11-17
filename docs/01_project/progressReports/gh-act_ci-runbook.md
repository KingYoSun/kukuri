# gh act CI failure 再現 Runbook
最終更新日: 2025年11月17日

## 概要
- GitHub Actions 上で再現が難しいジョブ（`format-check` や Docker ベースの TypeScript シナリオ）を Windows ホストから `gh act` で再実行するための手順をまとめる。
- ここでは「どうやって `gh act` でジョブを起動し、成果物・ログを採取するか」を中心に整理し、個別のトリアージ手順は既存 Runbook（例: [`docs/01_project/progressReports/nightly.trending-feed.md`](./nightly.trending-feed.md)）を参照する。

## 事前準備
- Docker Desktop 4.x で Linux コンテナを有効化し、`docker info` でデーモンに接続できることを確認する。
- GitHub CLI 2.60 以降と `gh act` 拡張をインストール済みであること（`gh --version`, `gh act --help` の実行で確認）。
- Windows ホストでは Runbook どおり `cmd.exe /c "corepack enable pnpm"` → `cmd.exe /c "corepack pnpm install --frozen-lockfile"` を一度実行し、`scripts/test-docker.ps1` が参照する `node_modules/.modules.yaml` を生成しておく。
- `tmp/act-artifacts/` を作成し（`mkdir -p tmp/act-artifacts`）、`--artifact-server-path` の出力先として使う。`actions/upload-artifact` を含むジョブは `ACTIONS_RUNTIME_TOKEN` が無いため、このオプションを渡さないと失敗する。
- 初回実行時は `docker volume ls | grep kukuri-` でキャッシュボリュームが未作成であることを確認し、完了後に `docker run --rm -v volume:/data alpine du -sh /data` で容量を把握しておくと `gh act` での再現コストを見積もりやすい。

## 共通オプション
`gh act` でいずれのジョブを再現する場合も、以下のオプションをベースにする。

| オプション | 目的 |
| --- | --- |
| `--bind` | ホストのワークスペースをコンテナへバインドし、`.git` や `pnpm-store` を丸ごとコピーする時間を削減する。 |
| `--container-options "--user 0"` | ジョブ内で `apt-get` や `npm install -g pnpm` を実行できるよう root で起動する。 |
| `--container-options "--privileged"` | Docker in Docker を使うジョブ（`scripts/test-docker.sh ts ...`）でホストの Docker ソケットをそのまま操作できるようにする。 |
| `-P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest` | `actions/setup-node` など JavaScript アクションを実行できる包括的なランナーイメージを利用する。 |
| `--artifact-server-path tmp/act-artifacts` | `actions/upload-artifact` をローカルパスへアップロードさせる。Nightly の `*-logs` 収集時に必須。 |

コマンド雛形:

```bash
gh act --bind \
  --workflows .github/workflows/<workflow>.yml \
  --job <job-id> \
  --container-options "--user 0<必要なら + --privileged>" \
  -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest \
  --artifact-server-path tmp/act-artifacts \
  > tmp/gh-act-<job-id>.log 2>&1
```

`tmp/gh-act-*.log` へ必ずリダイレクトし、GitHub Actions のログと同じ粒度で保存する。Runbook では該当ログを貼ったうえで triage 手順を記録する。

## Format Check (`.github/workflows/test.yml: format-check`)
- 目的: Rust/CLI/TypeScript/Docs のフォーマットチェックと `scripts/check_date_format.py` をローカルでグリーンにする。
- コマンド:

```bash
gh act --bind \
  --workflows .github/workflows/test.yml \
  --job format-check \
  --container-options "--user 0" \
  -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest \
  > tmp/gh-act-format-check.log 2>&1
```

- `--privileged` は不要。`npm install -g pnpm@9` を root で実行するため `--user 0` だけ付与する。
- 成功時は `tmp/gh-act-format-check.log` の末尾に以下が並ぶ。
  - `All checked dates use YYYY年MM月DD日 format.`（ドキュメント日付チェック）
  - `cargo fmt -- --check` / `pnpm format:check` の成功ログ
  - `Job succeeded`
- 参考ログ: `tmp/gh-act-format-check.log`（2025-11-17実行、4分47秒）

## TypeScript Docker シナリオ (`nightly.yml: profile-avatar-sync`)
- 目的: `scripts/test-docker.sh ts --scenario profile-avatar-sync` の Docker/ts 系ジョブを Windows から `gh act` で再現する。
- このジョブは Docker Compose を起動するため `--container-options "--privileged --user 0"` を必須とし、`Dockerfile.test` の `RUN chmod +x scripts/docker/ts-test-entrypoint.sh` で付与された実行権限を利用して `ts-test` コンテナを起動する。
- コマンド:

```bash
mkdir -p tmp/act-artifacts
gh act --bind \
  --workflows .github/workflows/nightly.yml \
  --job profile-avatar-sync \
  --container-options "--privileged --user 0" \
  -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest \
  --artifact-server-path tmp/act-artifacts \
  > tmp/gh-act-profile-avatar-sync.log 2>&1
```

- `tmp/gh-act-profile-avatar-sync.log` には以下が出力される。
  - `Scripts/test-docker.sh` による `test-runner` / `ts-test` イメージビルドログ
  - `pnpm vitest run ...` のテスト結果（`Test Files 3 passed` / `Tests 18 passed`）
  - `actions/upload-artifact` による `profile-avatar-sync-logs.zip` のアップロード（`--artifact-server-path` を指定しないと `ACTIONS_RUNTIME_TOKEN` エラーになる）
- 生成物:
  - `tmp/logs/profile_avatar_sync_<timestamp>.log`
  - `test-results/profile-avatar-sync/*.json`
  - `tmp/act-artifacts/profile-avatar-sync-logs`（zip）
- 参考ログ: `tmp/gh-act-profile-avatar-sync.log`（2025-11-17実行、15秒）
- Nightly ID/artefact 対応表は [`docs/01_project/progressReports/nightly.index.md`](./nightly.index.md#nightlyprofile-avatar-sync) を参照。

## トラブルシューティングメモ
- `actions/upload-artifact` が `Unable to get the ACTIONS_RUNTIME_TOKEN env variable` で失敗する場合は、本 Runbookのとおり `--artifact-server-path tmp/act-artifacts` を指定し、アップロード先フォルダを事前に作成する。
- `scripts/test-docker.sh` の `ts` シナリオが `permission denied` で停止する場合は、`Dockerfile.test` に `chmod +x scripts/docker/ts-test-entrypoint.sh` が入っている最新イメージを使っているか確認し、`docker compose -f docker-compose.test.yml build test-runner ts-test` を再実行する。
- Docker Compose 実行中に `volume "<name>" already exists ...` が表示される場合は `docker volume rm kukuri-pnpm-store` などで旧ボリュームを削除し、`pnpm-store` のキャッシュが別プロジェクトと衝突しないようにする。
- TypeScript のテストや ESLint で `act(...)` 警告が出た場合は、`tmp/gh-act-<job>.log` を Runbook に添付した上で、`docs/01_project/activeContext/tasks/status/in_progress.md` の該当メモを更新しておく。
