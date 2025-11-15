# Kukuri E2E テスト構築計画（GitHub Actions + Docker/Linux）

## 0. 再計画レビュー（2025年11月16日）

### 検証結果
- ルート直下に `e2e-tests` を新設して `pnpm init` する案は、既存ワークスペース（`kukuri-tauri/pnpm-workspace.yaml` で `.` のみ定義）と整合せず、依存解決とロックファイルが二重化するため実行不能。
- 提示されていた `wdio.conf.js` は JavaScript かつポート 4444 固定で、Tauri v2 の WebDriver 要件（`allowlist.automation` 有効化、4445 ポート、`tauri-driver` のライフサイクル管理、msedgedriver 連携）が欠けていた。
- GitHub Actions の手順が `pnpm install` をリポジトリ直下で呼び出し、`src-tauri` への `working-directory` も不正（`kukuri-tauri/src-tauri` が正）。さらに既存の `./scripts/test-docker.ps1` / `docker-compose.test.yml` を無視しており、Windows 開発者が再現できない。
- E2E シナリオで使用する `sqlite` シード、P2P ブートストラップ、ログ/artefact の採取場所が定義されておらず、Nightly Runbook との接続も曖昧。
- 既存ドキュメント（`docs/03_implementation/e2e_test_*`）と重複した説明が散在し、差分だけが分からない状態だった。

### 改訂方針
- E2E 関連の依存とソースはすべて `kukuri-tauri` 配下（`tests/e2e` ディレクトリ）に集約し、既存 pnpm ワークスペースで管理する。
- Tauri v2 が推奨する TypeScript ベースの WebDriverIO 設定を採用し、`tauri-driver`（4445/TCP）と `msedgedriver`（Windows）の両方を明示的に制御する。
- Docker テストランナーと `./scripts/test-docker.{ps1,sh}` に `SCENARIO=desktop-e2e` を追加し、ローカル（Windows）と GitHub Actions の両方が同じ経路で実行されるようにする。
- `test-results/desktop-e2e` と `tmp/logs/desktop-e2e` を標準化して artefact を Nightly / GA へ送る。
- 既存ガイド（`docs/03_implementation/e2e_test_setup.md` / `e2e_test_stabilization.md`）は前提知識として参照し、このドキュメントでは GitHub Actions 対応に必要な差分のみを記述する。

## 1. ゴール
1. `Key import → フィード読込 → 投稿作成` を網羅する最小限のデスクトップ E2E シナリオを自動化し、失敗時に再現ログとスクリーンショットを収集できること。
2. ローカル（Windows/WSL）では `./scripts/test-docker.ps1 e2e` で、CI では GitHub Actions 上の Linux コンテナで同一シナリオを実行できること。
3. `test.yml` / `nightly.yml` に `desktop-e2e` ジョブを追加し、`test-results/desktop-e2e` および `tmp/logs/desktop-e2e` を artefact 化する。

## 2. 前提・参照ドキュメント
- セットアップとトラブルシューティングは `docs/03_implementation/e2e_test_setup.md` と `e2e_test_stabilization.md` を参照。
- テスト実行は Windows では必ず `./scripts/test-docker.ps1`（PowerShell）、Linux/macOS では `./scripts/test-docker.sh` を経由する。
- Docker ベースのビルド環境（`Dockerfile.test`、`docker-compose.test.yml`）には `libwebkit2gtk-4.1-dev` など Tauri 依存が既に入っている。

## 3. 実装ステップ

### 3.1 `kukuri-tauri` 内での E2E モジュール整備
1. 依存追加  
   ```bash
   cd kukuri-tauri
   pnpm add -D webdriverio @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter \
     @wdio/types @wdio/globals ts-node tsconfig-paths
   ```
   - Windows で `tauri-driver` を使うため Rust 側では `cargo install tauri-driver --locked` と `msedgedriver-tool`（`docs/03_implementation/e2e_test_setup.md` 参照）を実施。

2. `package.json` へスクリプトを追加  
   ```jsonc
   {
     "scripts": {
       "e2e:seed": "node ./tests/e2e/tools/seed-db.cjs",
       "e2e:build": "pnpm e2e:seed && pnpm tauri build --debug --no-bundle",
       "e2e": "wdio run ./tests/e2e/wdio.desktop.ts",
       "e2e:ci": "xvfb-run -a wdio run ./tests/e2e/wdio.desktop.ts"
     }
   }
   ```
   - `e2e:seed` は `testdata/e2e_seed.db` を `src-tauri/data/kukuri.db` にコピーする Node スクリプト。
   - `xvfb-run` は CI 専用（ローカル Windows はネイティブ WebView2 を使用）。

3. ディレクトリ構造  
   ```
   kukuri-tauri/
     tests/
       e2e/
         fixtures/
           account.json          # 既知の秘密鍵と初期トピック
           trending_feed.json    # 安定した投稿リスト
         helpers/
           tauriDriver.ts        # tauri-driver プロセス管理
           waitForAppReady.ts    # #root レンダリング待機
         specs/
           app.smoke.spec.ts
         wdio.desktop.ts
         tsconfig.json
         tools/seed-db.cjs
   ```

4. `wdio.desktop.ts`（抜粋）  
   ```typescript
   import { join } from 'node:path';
   import type { Options } from '@wdio/types';
   import { startDriver, stopDriver } from './helpers/tauriDriver';

   export const config: Options.Testrunner = {
     runner: 'local',
     hostname: '127.0.0.1',
     port: 4445,
     specs: ['./tests/e2e/specs/**/*.spec.ts'],
     maxInstances: 1,
     logLevel: 'info',
     waitforTimeout: 15000,
     connectionRetryTimeout: 120000,
     connectionRetryCount: 2,
     capabilities: [{
       browserName: 'tauri',
       'tauri:options': {
         application: process.env.TAURI_E2E_APP_PATH
           ?? join(process.cwd(), 'src-tauri/target/debug/kukuri-tauri')
       }
     }],
     reporters: ['spec', ['@wdio/spec-reporter', { showPreface: false }]],
     framework: 'mocha',
     mochaOpts: { ui: 'bdd', timeout: 60000 },
     onPrepare: async () => {
       if (process.env.E2E_SKIP_BUILD !== '1') {
         await execa('pnpm', ['e2e:build'], { stdio: 'inherit' });
       }
     },
     beforeSession: async () => { await startDriver(); },
     onComplete: async () => { await stopDriver(); }
   };
   ```
   - `startDriver` では `tauri-driver --port 4445` を起動し、Windows の場合 `--native-driver <msedgedriver.exe>` を付与。
   - `beforeSuite` 内で `waitForAppReady()`・`seedFixture()` を呼び、`data-testid` セレクタ（`docs/03_implementation/e2e_test_stabilization.md` 参照）を利用。

5. `specs/app.smoke.spec.ts` の最低カバレッジ  
   - 鍵インポート UI（`data-testid="key-import-button"`）でフィクスチャの秘密鍵を入力 → 成功トースト表示。
   - トレンドタブ選択後、`tests/e2e/fixtures/trending_feed.json` の 1 件目と一致するタイトルが描画されること。
   - 投稿フォームでメッセージ送信 → ローカルタイムライン先頭に即時反映、`SyncStatusIndicator` が "Connected" へ遷移。

### 3.2 `tauri.conf.json` と `tauri-driver` 設定
- `kukuri-tauri/src-tauri/tauri.conf.json` に以下を追加し、E2E 実行時のみ自動化を許可:
  ```jsonc
  {
    "tauri": {
      "allowlist": {
        "all": true,
        "automation": true
      }
    },
    "app": {
      "windows": [
        { "title": "kukuri-tauri", "width": 1024, "height": 768 }
      ]
    }
  }
  ```
- リリースビルドでは `TAURI_ALLOW_AUTOMATION=0` を既定にし、`e2e:build` のみ `TAURI_ALLOW_AUTOMATION=1` を付与。
- `Dockerfile.test` に `xvfb` と `tauri-driver` を追加:
  ```dockerfile
  RUN apt-get update && apt-get install -y xvfb && rm -rf /var/lib/apt/lists/*
  RUN cargo install tauri-driver --locked
  ```
- Windows/ローカルでは `msedgedriver-tool` を `scripts/install-dev-tools.ps1` に組み込み、`C:\Users\<user>\kukuri\msedgedriver.exe` を既定パスとする。

### 3.3 データベースとフィクスチャ
- `kukuri-tauri/testdata/e2e_seed.db` を作成（`sqlx migrate run` 後、`topics`・`posts` テーブルへ最小データを投入）。
- `tests/e2e/tools/seed-db.cjs` で `src-tauri/data/kukuri.db` を上書きし、毎回クリーンな状態を保証。
- 追加の制御コマンド:
  - `src-tauri/src/commands/e2e.rs` に `reset_state` / `inject_trending_fixture` を実装（`feature = "test"` or `cfg!(feature = "test")` で限定）。
  - フロントでは `window.__KUKURI_E2E__` を提供し、テストから `await browser.execute()` で呼び出せるようにする。

### 3.4 Docker / `scripts/test-docker` 連携
1. `scripts/docker/run-smoke-tests.sh` 冒頭にブロックを追加:
   ```bash
   if [[ "${SCENARIO:-}" == "desktop-e2e" ]]; then
     /app/run-desktop-e2e.sh
     exit $?
   fi
   ```
2. 新規 `scripts/docker/run-desktop-e2e.sh`:
   ```bash
   #!/bin/bash
   set -euo pipefail
   cd /app/kukuri-tauri
   pnpm e2e:build
   xvfb-run -s "-screen 0 1280x720x24" pnpm e2e || rv=$?
   mkdir -p /app/test-results/desktop-e2e /app/tmp/logs/desktop-e2e
   cp tests/e2e/output/*.json /app/test-results/desktop-e2e/ || true
   cp tests/e2e/output/*.png /app/tmp/logs/desktop-e2e/ || true
   exit ${rv:-0}
   ```
   - `tests/e2e/output` には WDIO の `afterTest` で保存したスクリーンショット/JSON レポートを配置。
3. `scripts/test-docker.ps1` / `scripts/test-docker.sh` に `Command = "e2e"` を追加し、内部で  
   `SCENARIO=desktop-e2e docker compose -f docker-compose.test.yml run --rm test-runner` を呼び出す。  
   オプション `-NoBuild` や `-Fixture` は他シナリオと共通で扱う。
4. 生成物パス:
   - ログ: `tmp/logs/desktop-e2e/<timestamp>.log`（PowerShell 側で `Tee-Object`）。
   - WDIO レポート: `test-results/desktop-e2e/<timestamp>-report.json`
   - スクリーンショット: `test-results/desktop-e2e/screenshots/<spec>/<case>.png`

### 3.5 GitHub Actions への組み込み
- `.github/workflows/test.yml` に新ジョブ `desktop-e2e` を追加:
  ```yaml
  desktop-e2e:
    name: Desktop E2E (Docker)
    runs-on: ubuntu-latest
    needs: [docker-test]
    steps:
      - uses: actions/checkout@v4
      - name: Run desktop E2E via Docker harness
        shell: pwsh
        run: ./scripts/test-docker.ps1 e2e
      - name: Upload E2E artefacts
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: desktop-e2e-${{ github.run_id }}
          path: |
            test-results/desktop-e2e
            tmp/logs/desktop-e2e
  ```
- `nightly.yml` にも同じステップを追加し、artefact 名を `nightly.desktop-e2e-logs` / `nightly.desktop-e2e-reports` として `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` へ反映。
- テストサマリ `test-summary` には `needs: [..., desktop-e2e]` を追加し、失敗時は即座に検知できるようにする。

### 3.6 ローカル実行フロー（Windows 開発者）
1. `corepack enable pnpm`（Runbook に追記済みの手順を流用）。
2. `cargo install tauri-driver --locked`、`cargo install --git https://github.com/chippers/msedgedriver-tool`。
3. `msedgedriver-tool` をリポジトリ直下で実行し `msedgedriver.exe` を配置。
4. `./scripts/test-docker.ps1 e2e` を実行。内部で Docker コンテナが `pnpm e2e:ci` を起動するため、ホスト側で WebView2 を開く必要はない。
5. ネイティブで検証したい場合は
   ```
   pnpm --dir kukuri-tauri e2e:build
   tauri-driver --native-driver "$PWD/msedgedriver.exe"
   pnpm --dir kukuri-tauri e2e
   ```
   を 2 ターミナルで実行する。

## 4. リスクとフォローアップ
- **UI 変更に伴うセレクタ崩れ**: `data-testid` をドキュメント化 (`docs/03_implementation/e2e_test_stabilization.md`) し、変更時は E2E も同時に更新する運用を徹底。
- **P2P 実ネット依存**: すべての E2E シナリオは `tests/e2e/fixtures` のスタブと `KUKURI_FORCE_LOCALHOST_ADDRS=1` を用いたローカルブートストラップで完結させる。外部ノードへの接続は禁止。
- **CI 時間増大**: 1 シナリオ 6〜7 分を想定。`docker-test` と平行に動かすとホスト負荷が高いため `needs: docker-test` でシリアル化。将来的に並列化する場合はキャッシュ済みバイナリを `actions/cache` で共有する。
- **msedgedriver の更新**: Windows ホストで WebView2 が更新された場合は `msedgedriver-tool` を再実行する手順を `docs/03_implementation/windows_test_docker_runbook.md` に追記する。

以上の改訂により、GitHub Actions で再現可能な E2E テストパイプラインとローカル/CI 共有の Runbook を整備できる。
