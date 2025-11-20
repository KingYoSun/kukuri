# P2P Mainline Runbook
最終更新: 2025年11月12日

## 1. 目的
- Mainline DHT を有効にした P2P ネットワークの運用手順と統合テスト実行フローを共有する。
- 開発環境と CI の両方で同一手順を踏めるよう、必要な環境変数・ログ設定・トラブルシューティングを整理する。

## 2. 前提条件
- `ENABLE_P2P_INTEGRATION=1` を設定し、統合テストを明示的に許可する。
- `KUKURI_BOOTSTRAP_PEERS="node_id@host:port,...`"` で有効なブートストラップノードを指定する。
- 必要に応じて `KUKURI_IROH_BIN` で利用する `iroh` バイナリのパスを上書きできる（未設定時は PATH を参照）。
- Windows 環境では PowerShell 用スクリプト `./scripts/start-bootstrap-nodes.ps1` を用い、テスト前にノード群を起動する。

環境変数で `KUKURI_BOOTSTRAP_PEERS` を指定した場合、設定画面のブートストラップパネルは読み取り専用となり、適用済みノードとソース種別（env/user/bundle/n0）が UI に表示される。ローカルで値を確認したい場合は `pnpm tauri dev` → Settings→「ブートストラップ設定」を開く。
### 2.1 推奨環境変数セット（例）
```powershell
$env:ENABLE_P2P_INTEGRATION = "1"
$env:KUKURI_BOOTSTRAP_PEERS = "k51qzi5uqu5dl@127.0.0.1:44001,k51qzi5uqu5dn@127.0.0.1:44002"
$env:RUST_LOG = "info,iroh_tests=debug"
```
- メモ: `RUST_LOG` を設定しない（または `mainline::rpc::socket` を含まない）状態で `pnpm tauri dev` を起動した場合、`mainline::rpc::socket` には既定で `=error` ディレクティブが付与され WARN が抑制される。必要に応じて `$env:RUST_LOG = "info,mainline::rpc::socket=debug,..."` のように明示指定すると従来通り mainline WARN を確認できる。

## 3. テスト構成概要
- Phase 5 で Rust 統合テストを `kukuri-tauri/src-tauri/tests` 配下のテストバイナリへ完全移行済み。
  - `p2p_gossip_smoke.rs`: Gossip 経路のスモークテスト。Phase 5 で `tests/` 配下に再編したテストバイナリ。
  - `p2p_mainline_smoke.rs`: Mainline DHT 経路のスモークテスト。ブートストラップ接続とルーティングの健全性を検証する。
  - `tests/integration/test_p2p_mainline.rs`: P2PService Builder と `DiscoveryOptions` の回帰テスト。2025年10月25日にカスタムディスカバリ override ケースを追加し、Mainline DHT フローの構成が自動検証されるようにした。
  - `tests/integration/offline` : OfflineService と再索引ジョブの結合テスト（2025年10月30日追加）。`recovery.rs` で再索引レポートとキュー再投入の挙動を確認する。
  - `tests/integration/event/manager` : EventManager ? AppHandle ?????????2025?10?30?????`cargo test --package kukuri-tauri --test event_manager_integration` ??????
  - `scripts/docker/run-smoke-tests.sh` / `scripts/test-docker.{sh,ps1}` は両テストを順次実行する構成に統一済みで、旧バイナリへのフォールバックは存在しない。

### 3.1 共通ユーティリティの活用
- Phase 4 でテスト支援コードを `application/shared/tests/p2p` へ集約済み。新しい `p2p_*_smoke` からも同ユーティリティを利用する。  
- Gossip 経路で利用する DefaultTopicsRegistry や EventPublisher も `application/shared` へ移動済み。自前で複製せず共有モジュールを参照して重複を避ける。  
- 新しいシナリオを追加する場合は `src-tauri/tests/common` を再利用し、smoke テストに倣って `ENABLE_P2P_INTEGRATION` とブートストラップ待機処理を組み込む。

### 3.2 EventGateway メトリクスと結合テスト（2025年10月25日追加）
- `infrastructure::event::metrics` で Gateway API（受信／Publish／Reaction／Metadata／削除／Disconnect）の成功・失敗回数と直近タイムスタンプを記録する仕組みを導入。`LegacyEventManagerGateway` すべてのパスが `metrics::record_outcome` を経由する。
- メトリクスの動作と Gateway の DI を確認するには `cargo test --package kukuri-tauri --test test_event_service_gateway -- --nocapture` を実行し、`tests/integration/test_event_service_gateway.rs` を通過させる。失敗時は `metrics::snapshot()`（`presentation/commands` 追加予定）で現在値を取得し、`incoming.failures` 等のカウンタから再現手順を追跡する。
- 2025年11月13日: `state.rs` で `Arc<dyn EventGateway>` と `Arc<dyn P2PServiceTrait>` を注入し、`presentation/handlers/{event_handler,p2p_handler}.rs` を Legacy 実装から切り離した。`cargo test --package kukuri-tauri --all-features` と `cargo test --package kukuri-tauri --test p2p_mainline_smoke -- --nocapture --test-threads=1` の結果を `tmp/logs/cargo-test-kukuri-tauri_di_20251113.log` に保存し、`tests/integration/topic_create_join.rs` の trait モックで join/create が Gateway → Gossip/Mainline に流れることを再確認する。

### 3.3 dead_code クリーンアップと metrics export（2025年11月19日）
- `rg '#\[allow(dead_code)' -g '*.rs'` で 7 件あった `#![allow(dead_code)]` を棚卸しし、`application` / `shared` / `presentation` / `infrastructure` / `domain` の各 `mod.rs` から削除。未参照だった `src-tauri/src/modules/**/*` はディレクトリごと撤去し、Phase 5 backlog から除外した。ログ: `tmp/logs/dead_code_inventory_20251119-053142.log`（削除前） / `tmp/logs/dead_code_inventory_20251119-053645.log`（削除後）。
- `p2p_metrics_export` が `test_support` ではなく本番コンポーネント（`TopicMetricsRepository` / `SqliteRepository` / `ConnectionPool` / `AppConfig`）をそのまま利用できるよう `lib.rs` で対象型を再エクスポート。`./scripts/test-docker.ps1 rust`（`tmp/logs/test_docker_rust_20251119-055648.log`）で bin 含む 190 テスト成功を確認した。
- `./scripts/test-docker.ps1 lint` に含まれる `cargo fmt -- --check` / `cargo clippy` は Rust 1.86.0 の `rustfmt`/`rustc` が `application/shared/tests/mod.rs` のマルチバイトコメント処理で panic（`tmp/logs/test_docker_lint_20251119-055329.log` / `tmp/logs/cargo_clippy_20251119-055408.log`）。コメントを ASCII + LF に変換済みだが、現 toolchain では再現するため Rustfmt/Clippy のワークアラウンドとして「rustup 更新待ち + `pnpm lint` / `pnpm format:check` を個別実行（`tmp/logs/pnpm_lint_20251119-055434.log`）」を記録している。

### 3.3 パフォーマンスハーネス（2025年10月31日追加）
- Phase 5 で `tests/performance/cache.rs`（OfflineService の save/list・キャッシュクリーニング）と `tests/performance/sync.rs`（OfflineReindexJob・sync_actions）の計測ケースを分割。`tests/common/performance/recorder.rs` に計測結果を JSON 化するユーティリティを追加した。
- 実行は `./scripts/test-docker.ps1 performance` または `./scripts/test-docker.sh performance` で行う。内部で `cargo test --test performance -- --ignored --nocapture` を呼び出し、成果物を `test-results/performance/*.json` に出力する。
- デフォルトの保存先は `KUKURI_PERFORMANCE_OUTPUT` 環境変数で上書き可能。CI で保持する場合は `test-results/performance` を artefact として収集する。反復計測時は JSON に含まれる `iterations` や `metrics.*_per_sec` を比較してリグレッションを検出する。

## 4. 実行手順
1. ブートストラップノードを起動（例: `./scripts/start-bootstrap-nodes.ps1 -ReplicaCount 3`）。
2. 上述の環境変数を設定。
3. テスト開始:
   ```powershell
   cargo test --package kukuri-tauri --test p2p_mainline_smoke -- --nocapture --test-threads=1
   ```
4. 重要シナリオのみを個別確認する場合:
   ```powershell
   cargo test --package kukuri-tauri --test p2p_gossip_smoke -- --nocapture --test-threads=1
   ```
5. Windows で DLL 解決に問題がある場合は下記の PowerShell コマンドで Docker 経由の統合テストを実行する。
   ```powershell
   ./scripts/test-docker.ps1 rust -Integration `
     -BootstrapPeers "node_id@127.0.0.1:11233" `
     -IrohBin "C:\tools\iroh.exe"
   ```
   - `-IntegrationLog` で `RUST_LOG` を上書き可能。
   - `integration` コマンド単体でも同様のオプションを利用できる（例: `./scripts/test-docker.ps1 integration -BootstrapPeers ...`）。
   - PowerShell 版スクリプトは `p2p_gossip_smoke` / `p2p_mainline_smoke` の双方を順次実行する。
6. EventManager ???????????????:
   ```powershell
   cargo test --package kukuri-tauri --test event_manager_integration -- --nocapture
   ```
   Docker ????????????
   ```powershell
   ./scripts/test-docker.ps1 rust -Test event_manager_integration
   ```

## 5. ログとトレース
- `support/logging.rs` で `tracing_subscriber` を初期化し、`iroh_tests` ターゲットでログを出力する。
- 期待ログ:
  - `binding endpoint` / `adding bootstrap node` / `connected to bootstrap` でノード初期化状況を把握。
  - `services joined topic` / `broadcasting` / `received` の流れが確認できれば通信経路が成立。
- 失敗時は `RUST_LOG=trace,iroh=info` に上げることで iroh 側の詳細ログを取得できる。

### 5.1 Topic Create Offline 再送ログ（Stage4, 2025年11月12日追加）
- 目的: `OfflineActionType::CREATE_TOPIC` が `topics_pending` テーブルに蓄積され、オンライン復帰後に `TopicService::mark_pending_topic_synced|failed` → `P2PService::join_topic` が実行されることを証跡化する。
- 手順:
  1. フロントエンドユニットテストで UI を再現  
     ```powershell
     cd $RepoRoot/kukuri-tauri
     $ts = Get-Date -Format 'yyyyMMdd-HHmmss'
     $log = \"../tmp/logs/topic_create_host_$ts.log\"
     npx pnpm vitest run `
       src/tests/unit/components/topics/TopicSelector.test.tsx `
       src/tests/unit/components/posts/PostComposer.test.tsx `
       src/tests/unit/components/layout/Sidebar.test.tsx `
       src/tests/unit/scenarios/topicCreateOffline.test.tsx `
       2>&1 | Tee-Object -FilePath $log
     ```
     - `TopicSelector` の「保留中のトピック」グループと、Scenario テストの `watchPendingTopic` 呼び出しを確認する。`Input` は `forwardRef` 化済みのため Radix の ref 警告は出力されない。
  2. Docker で Nightly シナリオを再現  
     ```powershell
     cd $RepoRoot
     ./scripts/test-docker.ps1 ts -Scenario topic-create [-NoBuild]
     ```
     - 結果は `tmp/logs/topic_create_<timestamp>.log` と `test-results/topic-create/<timestamp>-*.json`（TopicSelector/PostComposer/Sidebar/topicCreateOffline の 4 ファイル）に保存する。Nightly artefact 名は `topic-create-logs` / `topic-create-reports` を使用。
  3. `topics_pending` の状態を確認  
     ```powershell
     cd $RepoRoot/kukuri-tauri/src-tauri
     sqlite3 data/kukuri.db \"SELECT pending_id,status,synced_topic_id,error_message FROM topics_pending;\"
     ```
     - オフライン作成直後は `status='queued'`、同期済みは `status='synced'` と `synced_topic_id` が埋まる。再送失敗時は `status='failed'` と `error_message` を必ず確認する。
- 期待成果物: `../tmp/logs/topic_create_host_<timestamp>.log`, `tmp/logs/topic_create_<timestamp>.log`, `test-results/topic-create/<timestamp>-*.json`。Runbook 5章に記載された各ログを `phase5_ci_path_audit.md`（`nightly.topic-create.*` 行）と突き合わせる。

### 5.2 SyncStatusIndicator 再送メトリクス（Stage5, 2025年11月15日追加）
- 目的: Offline SyncEngine／Service Worker／`offline_actions` テーブルの再送フローを `metrics::record_outcome` に集約し、UI・Nightly artefact・Runbook で同じ再送メトリクスを参照できるようにする。
- 手順:
  1. TypeScript シナリオをカテゴリ別に実行  
     ```bash
     ./scripts/test-docker.sh ts --scenario offline-sync --offline-category topic
     ./scripts/test-docker.sh ts --scenario offline-sync --offline-category post --no-build
     ./scripts/test-docker.sh ts --scenario offline-sync --offline-category follow --no-build
     ./scripts/test-docker.sh ts --scenario offline-sync --offline-category dm --no-build
     # PowerShell: .\scripts\test-docker.ps1 ts -Scenario offline-sync -OfflineCategory topic [-NoBuild]
     ```
     - 出力: `tmp/logs/sync_status_indicator_stage4_{topic,post,follow,dm}_<timestamp>.log`（Vitest + `errorHandler SyncStatus.*` ログ）と `test-results/offline-sync/{topic,post,follow,dm}/${timestamp}-*.json`（カテゴリ別 JSON レポート）。Nightly artefact は `sync-status-indicator-topic` / `...-post` / `...-follow` / `...-dm` をそれぞれアップロードし、Runbook から直接リンクできるようにした。
  2. UI 側で確認する項目  
     - SyncStatusIndicator ポップオーバー内の「再送メトリクス」カードに `成功/失敗` 合計、`連続失敗数`、`直近の再送`（`jobId` / `reason` / `retryCount/maxRetries` / `backoff` / 実行時間 / 記録時刻）が表示される。
     - `scheduledRetry` が存在する場合は「次回 #<jobId> を <timestamp> に再送（n/m）」の警告が表示され、Service Worker の Backoff と一致する。
     - `errorHandler` の `SyncStatus.queue_snapshot` / `...pending_actions_snapshot` / `...retry_metrics_snapshot` が `tmp/logs/sync_status_indicator_stage4_*` に JSON を追記し、`sync_queue` / `offline_actions` / `offline_retry_metrics` の Nightly 計測値を UI から直接取得できる。
  3. バックエンド側の計測  
     - `presentation::commands::record_offline_retry_outcome` を通じて `infrastructure::offline::metrics::record_outcome` が呼ばれ、`OfflineRetryMetricsSnapshot`（`total_success` / `total_failure` / `consecutive_failure` / `last_*` フィールド）を更新する。
     - snapshot は `get_offline_retry_metrics` コマンド経由で取得でき、Vitest JSON には `retryMetrics` の状態が記録される。`tmp/logs/sync_status_indicator_stage4_{topic,post,follow,dm}_<timestamp>.log` にも `retry_outcome` ログ行が出力され、CI で triage 可能。
- 期待成果物: `tmp/logs/sync_status_indicator_stage4_{topic,post,follow,dm}_<timestamp>.log`, `test-results/offline-sync/{topic,post,follow,dm}/${timestamp}-*.json`。`phase5_ci_path_audit.md` と `nightly.sync-status-indicator.*` 行から参照する。

## 6. CI 統合ポイント
- GitHub Actions（`ci/rust-tests.yml`）では統合テスト専用ステップを追加し、以下を設定する:
  - `ENABLE_P2P_INTEGRATION=1`
  - `KUKURI_BOOTSTRAP_PEERS=memory://gha-bootstrap` （workflow 内でモックノード起動）
  - `RUST_LOG=info,iroh_tests=debug`
- テストジョブは `--test-threads=1` で直列化し、タイムアウトは 20 分に拡張する。
- フレーク発生時は GitHub Actions のログから `iroh_tests` をフィルタし、対象シナリオをピンポイントで再実行する。

### 6.1 Rustカバレッジ測定（Phase 5 Workstream B）
- ローカル/CI 共通コマンド: `./scripts/test-docker.sh coverage`（PowerShell 版も同名）。内部で `docker compose run rust-coverage` を実行し、`cargo tarpaulin --locked --all-features --skip-clean --out Json --out Lcov --output-dir /app/test-results/tarpaulin --timeout 1800` を採用する。
- 成果物: `test-results/tarpaulin/` に JSON と LCOV を出力し、スクリプト終了時に `docs/01_project/activeContext/artefacts/metrics/<timestamp>-tarpaulin.{json,lcov}` へ自動コピーする。
- 閾値: Phase 5 時点では参考値（2025年10月26日: 25.23%）。Phase 6 移行後に 50% / 70% を順次クリアし、CI では `tarpaulin --fail-under <target>` を段階適用する。
- Tarpaulin は ptrace を利用するため `rust-coverage` サービスに `SYS_PTRACE` 権限と `seccomp=unconfined` を付与済み。CI で同設定を反映する場合は GitHub Actions の `docker run` 手順で `--cap-add=SYS_PTRACE --security-opt seccomp=unconfined` を指定する。

### 6.2 P2Pメトリクス採取
- Phase 5 で `p2p_metrics_export` バイナリとラッパースクリプト `scripts/metrics/export-p2p.{sh,ps1}` を追加した。`--job p2p`（既定）は Gossip/Mainline スナップショットを `docs/01_project/activeContext/artefacts/metrics/<timestamp>-p2p-metrics.json` へ保存し、`--job trending` は `test-results/trending-feed/metrics/<timestamp>-trending-metrics.json` に `window_*` / `lag_ms` / `score_weights` / `topics[].score_24h` を含むレポートを出力する。
  ```bash
  ./scripts/metrics/export-p2p.sh --job trending --pretty --limit 50
  ./scripts/metrics/export-p2p.sh --job p2p --pretty
  ```
- PowerShell 版は `./scripts/metrics/export-p2p.ps1 -Job trending -Pretty` / `-Job p2p -Pretty` で同じ情報を取得できる。`--database-url` / `-DatabaseUrl` で SQLite の場所を指定し、`--output` / `-Output` で保存先を上書き可能。
- エクスポートされた JSON を `phase5_ci_path_audit.md` と紐づけ、`lag_ms` が 5 分未満・`metrics_count` > 0 であることを Nightly で確認する。P2P 側の JSON はこれまで同様 CI 進捗レポートへ添付する。
- 2025年11月11日以降、`./scripts/test-docker.sh ts --scenario trending-feed`（PowerShell 版は `.\scripts\test-docker.ps1 ts -Scenario trending-feed`）が `prometheus-trending` サービスを自動起動し、`tmp/logs/trending_metrics_job_stage4_<timestamp>.log` へ `curl http://127.0.0.1:9898/metrics` の内容と Prometheus ログを保存する。Summary Panel の Vitest と監視ログ採取を同一ジョブで確認できるため、Nightly では当該ログを Runbook と `phase5_ci_path_audit.md` に添付する。2025年11月12日更新では同ログを `test-results/trending-feed/prometheus/` 以下にも複製し、`nightly.yml` の `trending-metrics-prometheus` artefact としてダウンロードできるようにした。

### 6.3 iroh バイナリキャッシュ
- GitHub Actions では `actions/cache@v4` を利用し、`~/.cache/kukuri/iroh`（PowerShell 版は `%LocalAppData%\kukuri\iroh`）をキャッシュする。キーは `iroh-${{ runner.os }}-${{ hashFiles("scripts/install-iroh.ps1") }}` を推奨し、`scripts/install-iroh.{sh,ps1}` でキャッシュヒット時はダウンロードをスキップする。
- ローカル環境でも `./scripts/install-iroh.ps1 -UseCache`（PowerShell）または `./scripts/install-iroh.sh --use-cache` を使用することで同ディレクトリを再利用し、`docker` テスト前のセットアップ時間を短縮できる。

### 6.4 Nightly シナリオと artefact パス
`nightly.yml` の各ジョブは `phase5_ci_path_audit.md` / `phase5_user_flow_summary.md` から参照できるテスト ID とログパスを共有している。ログはすべて `tmp/logs/<scenario>_<timestamp>.log` 形式で保存し、該当 artefact 名を GitHub Actions からダウンロードする。

| Test ID (`nightly.yml` job) | シナリオ/コマンド | 主要 artefact | `tmp/logs` パス |
| --- | --- | --- | --- |
| `trending-feed` | `./scripts/test-docker.sh ts --scenario trending-feed --no-build`（PowerShell 版あり） | `trending-feed-reports`（`test-results/trending-feed/*.json`）<br>`trending-metrics-logs`（Prometheus `curl` スナップショット）<br>`trending-metrics-prometheus`（`test-results/trending-feed/prometheus/*`） | `tmp/logs/trending-feed/<timestamp>.log` と `tmp/logs/trending-feed/latest.log`<br>`tmp/logs/trending_metrics_job_stage4_<timestamp>.log` |
| `profile-avatar-sync` | `./scripts/test-docker.sh ts --scenario profile-avatar-sync` | `profile-avatar-sync-logs` | `tmp/logs/profile_avatar_sync_<timestamp>.log` |
| `sync-status-indicator` | `./scripts/test-docker.sh ts --scenario offline-sync --offline-category {topic,post,follow,dm}` | `sync-status-indicator-topic` / `-post` / `-follow` / `-dm` | `tmp/logs/sync_status_indicator_stage4_{topic,post,follow,dm}_<timestamp>.log` / `test-results/offline-sync/{topic,post,follow,dm}/${timestamp}-*.json` |
| `user-search-pagination` | `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build` | `user-search-pagination-logs`（Vitest stdout）<br>`user-search-pagination-log-archive`（`test-results/user-search-pagination/logs/*.log`）<br>`user-search-pagination-reports`（`test-results/user-search-pagination/reports/*.json`）<br>`user-search-pagination-search-error`（`test-results/user-search-pagination/search-error/<timestamp>-search-error-state.json`：2文字未満→補助検索→SearchErrorState） | `tmp/logs/user_search_pagination_<timestamp>.log` |
| `post-delete-cache` | `./scripts/test-docker.sh ts --scenario post-delete-cache`<br>`.\scripts\test-docker.ps1 ts -Scenario post-delete-cache` | `post-delete-cache-logs`（`tmp/logs/post_delete_cache_<timestamp>.log`）<br>`post-delete-cache-reports`（`test-results/post-delete-cache/<timestamp>-*.json`：`useDeletePost` / `postStore` / `PostCard` 系4ファイル） | `tmp/logs/post_delete_cache_<timestamp>.log`（PowerShell 版は `tmp/logs/post_delete_cache_YYYYMMDD-HHMMSS.log` が自動採取される） |

各ログのダウンロード先は Nightly artefact 一覧に表示されるため、Runbook 上で該当 ID を指定すれば再現経路・リカバリー手順を即時に辿れる。`phase5_ci_path_audit.md` にも同じ Test ID／パスを記載し、Ops/CI Onboarding での証跡共有を容易にする。
## 7. トラブルシューティング
- **`STATUS_ENTRYPOINT_NOT_FOUND`**: Windows で iroh バイナリの依存 DLL が見つからない場合に発生。`KUKURI_IROH_BIN` を明示し、`PATH` に `libssl` 等が含まれているか確認。Docker 実行で迂回可能。
- **ブートストラップ接続失敗**: `KUKURI_BOOTSTRAP_PEERS` の NodeId/ポートを再確認し、ファイアウォールで該当ポートを開放する。
- **テストタイムアウト**: `support/bootstrap.rs` の `DEFAULT_JOIN_TIMEOUT` / `DEFAULT_EVENT_TIMEOUT` を一時的に延長し、`RUST_LOG=debug` でどこまで進んでいるかを追跡。根治策はブートストラップノードのキャパシティ調整。
- **ログが出力されない**: 既に別のサブスクライバが設定されている可能性。テスト起動前に `RUST_LOG` を設定し、`tracing_subscriber::fmt` 初期化が一度だけ呼ばれているかを確認。

## 8. 今後の TODO
- NIP-01/10/19/30078 の受信フィルタ結合テストを Phase 6 で追加し、9章の検証ポリシーを自動検証する。
- Mainline DHT フォールバックノードの自動ローテーション（署名付きリストと稼働監視）の実装方針を検討。

- 再接続・再索引シナリオ用の `recovery.rs` を実装し、OfflineService との結合テストを追加。
- iroh バイナリのキャッシュ戦略（GitHub Actions 用）を整備し、ダウンロード時間を短縮する。

## 9. NIP 準拠検証ポリシー（確定版）

詳細仕様と背景は `docs/03_implementation/nostr_event_validation.md` を参照。ここでは運用時に参照すべき Pass/Fail 条件と担当レイヤをまとめる。

| 対象 | Pass 条件 | Fail 条件 | 主担当 |
| --- | --- | --- | --- |
| **NIP-01（基本整合性）** | `id/pubkey/sig` は 64/64/128 桁 hex、`id` 再計算一致、`created_at` は `now ±10分` 以内、JSON スキーマ妥当 | hex 形式不正、署名再計算不一致、タイムスタンプ乖離、シリアライズ失敗 | アプリケーション層（`EventGateway`/`EventService`） |
| **NIP-10（返信タグ）** | `e`/`p` タグは 64hex または bech32（`note`/`nevent`/`npub`/`nprofile`）、`relay_url` は空 or `ws[s]://`、`marker` は `root`/`reply`/`mention` のみ、`root`/`reply` は最大 1 件、`reply` 出現時に `root` も存在 | marker 未定義、`relay_url` が http 等、`root`/`reply` 重複、`reply` 単独、bech32 不整合 | アプリケーション層 |
| **NIP-19（bech32 TLV）** | `npub`/`nprofile` は tag=0 32byte 公開鍵、relay tag ≤16・ASCII・`ws[s]://`、`nevent` は tag=0=event ID・tag=2=author(32byte 任意)・tag=3=kind(4byte BE) | TLV 長超過、非 ASCII、relay 上限超過、`hrp` 不一致、未定義 tag が 1KB 超 | アプリケーション層 |
| **kind:30078（Parameterised Replaceable Event）** | `kind`=30078、`["d","kukuri:topic:<slug>:post:<revision>"]` が必須（`slug` は `TopicId` に準拠、`revision` は base32/UUID 文字列）、`["k","topic-post"]` 固定、`["t","topic:<slug>"]` 単一指定、`["a","30078:<pubkey>:kukuri:topic:<slug>:post:<revision>"]` 一致、`content` は JSON `{body,attachments,metadata}` で 1MB 未満、時系列で最新のみ有効 | `d` 欠如や形式不正、`k`/`t`/`a` の欠落・不一致、複数トピック指定、`content` サイズ超過、古い timestamp が最新を上書き | アプリケーション層 |
| **共通制限** | `content` ≤ 1MB、`tags` ≤ 512、UTF-8 妥当 | サイズ超過、非 UTF-8、未知種別での重大フォーマット崩れ | アプリケーション層 |

### 運用メモ
- インフラ層（`IrohGossipService`）は JSON デコード失敗やシグネチャ検証失敗といった明確な異常値を受信時に破棄し、詳細な NIP 判定はアプリケーション層へ移譲する。
- 検証失敗時は `EventGateway` が `AppError::ValidationError` を発行し、`metrics::record_receive_failure()` に記録する。Offline リプレイからの除外や隔離は `nostr_event_validation.md` の手順に従う。
- 契約テストは `kukuri-tauri/src-tauri/tests/contract` に NIP-10/NIP-19/kind30078 のサンプルベクトルを追加し、P2P 経路の回帰は `tests/integration/p2p_*` で担保する。Docker ベースの統合テスト実行時には 9章の条件に違反したイベントが無効化されているかをログとメトリクスで確認する。
  - JSON フィクスチャ: `tests/testdata/nip10_contract_cases.json` / `nip19_contract_cases.json` / `kind30078_contract_cases.json` を利用し、`case_id`・`description`・`expected` を揃える。
- kind:30078 の PRE は `kukuri:topic:<slug>:post:<revision>` 単位で最新を採用する。再投稿検知時は `metadata.edited=true` を付与し、旧イベントは Offline 再索引ジョブが自動的に破棄する。
- `ValidationFailureKind` に応じた `receive_failures_by_reason` を監視し、異常があれば WARN ログの `reason` と Offline レポートの `SyncStatus::Invalid` 記録を突合して原因を特定する。レポートは `offline://reindex_complete` イベントで取得できる。
- 各テストの配置と責務は `docs/03_implementation/nostr_event_validation.md` 5.1節のマッピング表を参照。Runbook 更新時は対応するテスト名も必ず記録する。

## 4. Profile Avatar Sync（2025年11月12日更新）
- `profile_avatar_sync` コマンドは `npub` / `known_doc_version` に加えて `source` / `requested_at` / `retry_count` / `job_id` を受け取り、同期結果を `cache_metadata`（`cache_key=doc::profile_avatar::<npub>`）へ TTL 30 分で保存する。`metadata.result` には `updated`・`currentVersion`・Blob 概要が入るため、Runbook から Doc/Blob のドリフトと Service Worker リトライ状況を追跡できる。
- フロントエンドは `useProfileAvatarSync` フックを常駐させ、Service Worker (`profileAvatarSyncSW.ts`) から受信したジョブを BroadcastChannel で処理する。失敗時は `retry_count` に応じて指数バックオフで再投入し、完了結果を `offlineApi.addToSyncQueue`（action_type=`profile_avatar_sync`）に記録して Ops UI/Runbook から参照できるようにした。`ProfileEditDialog` / `ProfileSetup` では保存後に `syncNow({ force: true, source: 'useProfileAvatarSync:manual' })` を呼び、Doc バージョンと `authStore` を即時更新する。
- 自動・手動検証フロー
  1. `pnpm vitest run src/tests/unit/components/settings/ProfileEditDialog.test.tsx src/tests/unit/components/auth/ProfileSetup.test.tsx src/tests/unit/hooks/useProfileAvatarSync.test.tsx src/tests/unit/workers/profileAvatarSyncWorker.test.ts`
  2. `./scripts/test-docker.sh ts --scenario profile-avatar-sync --service-worker`（PowerShell: `./scripts/test-docker.ps1 ts -Scenario profile-avatar-sync -ServiceWorker -NoBuild`）で Stage4 ログ `tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` と worker テスト結果を Nightly artefact `profile-avatar-sync-logs` に保存する。
  3. `./scripts/test-docker.ps1 rust -Test profile_avatar_sync`（Windows では `-NoBuild` 併用可）または `cargo test --package kukuri-tauri --test profile_avatar_sync`
- 失敗時は `kukuri-tauri/src-tauri/target/profile_avatars/doc.json`（Docker: `/app/kukuri-tauri/src-tauri/target/profile_avatars/doc.json`）と `blobs/` 配下のハッシュ、`cache_metadata` の `metadata.result`、`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` を突き合わせ、Service Worker の再送ログと Doc/Blob 差分を確認する。必要に応じて `rm -rf profile_avatars` → `cargo test --package kukuri-tauri --test profile_avatar_sync` を再実行し、`AppError::Storage` が消えるかを確認する。

### 4.4 鍵バックアップ/復旧（2025年11月17日追加）
1. **事前確認**
   - Settings > アカウントに「鍵管理」ボタンが表示されていること、`src/routes/settings.tsx` で `KeyManagementDialog` がレンダリングされていることを確認する。
   - `persistKeys.keyManagement` が localStorage に作成されることを DevTools で確認し、履歴がローテーション（最大20件）されることを確認する。
2. **バックアップ手順**
   1. 「鍵管理」→「エクスポート」タブで `秘密鍵を取得` を押し、`KeyManagementDialog` に `nsec` が表示されることを確認する。
   2. 「ファイルに保存」で `.nsec` をオフライン媒体へ保存し、必要に応じて「クリップボードにコピー」を利用する（30 秒以内に貼り付けて削除する）。
   3. `useKeyManagementStore` の履歴に `action: export / stage: fetch, save-file` が `status: success` で記録されていることを確認し、証跡として Runbook に貼り付ける。
3. **復旧手順**
   1. 「インポート」タブで `鍵ファイルを選択` → `.nsec` を読み込み、または手動入力欄に貼り付ける。
   2. `セキュアストレージに追加` を押し、`authStore.loginWithNsec(nsec, true)` が成功することを確認。完了後に `currentUser.npub` が切り替わっているか、リレー状態が更新されるかを Settings で確認する。
   3. 履歴に `action: import / status: success / metadata.source: file|manual` が記録されることを確認し、監査ログとして保存する。
4. **検証コマンド**
   - UI/ストア: `./scripts/test-docker.ps1 ts`（Vitest 全体に `KeyManagementDialog` / `keyManagementStore` のテストが含まれる）。
   - バックアップ/復旧契約: `./scripts/test-docker.ps1 rust -Test key_management`。
5. **障害対応メモ**
   - エクスポート失敗時は `src/lib/api/tauri.ts` の `exportPrivateKey` 呼び出し結果と `src-tauri/src/presentation/commands/auth_commands.rs` の `AppError` を突合し、`AuthService::export_private_key` で鍵が見つからない場合は `SecureStorage` の ledger を調査する。
   - インポート失敗時は `.nsec` の形式（`nsec1` 始まり）と `KeyManagementDialog` のバリデーション結果 (`status: cancelled|error`) を確認し、`useKeyManagementStore` の履歴と `errorHandler.log('KeyManagementDialog.handleImport', …)` をエビデンスとして Runbook に貼り付ける。
- **孤立コンポーネント監査（2025年11月19日）**: Settings > アカウント > 鍵管理ボタン経由で `KeyManagementDialog` を開いた際の `errorHandler.info/log('KeyManagementDialog.*')` テレメトリと `persistKeys.keyManagement` の履歴を採取し、Inventory 5.13 / `phase5_user_flow_summary.md` 1.5 / refactoring plan KPI「孤立コンポーネント0件」と突合する。Docker `./scripts/test-docker.ps1 ts`（UI）と `./scripts/test-docker.ps1 rust -Test key_management`（コマンド）を同日実行し、ログ採取時は `tmp/logs/key_management_<timestamp>.log` および Nightly `key-management` artefact へ保存する。


## 9. get_p2p_status API 拡張実装（2025年11月03日）
- `application::services::p2p_service::P2PStatus` に `connection_status: ConnectionStatus`（`connected` / `connecting` / `disconnected` / `error`）と `peers: Vec<PeerStatus>` を追加し、`presentation::handlers::p2p_handler::get_p2p_status` → `presentation::dto::p2p::P2PStatusResponse` 経由でフロントへ返却する。`PeerStatus` は Node ID・endpoint アドレス・最終観測時刻を含む。
- `p2pApi.getStatus` / `useP2PStore.refreshStatus` / `useP2P` を更新し、`connectionStatus`・`peers`・バックオフ関連フィールド（`statusBackoffMs` / `lastStatusFetchedAt` / `statusError` / `isRefreshingStatus`）をストアと `P2PStatus` コンポーネントへ反映する。UI はヘッダーに最終更新時刻と次回再取得目安、手動 `再取得` ボタン、エラーバナーを表示。
- 検証手順:
  1. `npx vitest run src/tests/unit/components/P2PStatus.test.tsx src/tests/unit/stores/p2pStore.test.ts src/tests/unit/hooks/useP2P.test.tsx` — バックオフ制御・新フィールド描画・手動リトライをフェイクタイマーで検証。
  2. `cargo test --package kukuri-tauri --lib application::services::p2p_service::tests`（または `cargo test`）— `connection_status` / `peers` 拡張後のフォールバックシナリオ（メトリクス欠落 → peers 参照）を確認。
  3. 手動動作確認: `pnpm tauri dev` で起動し、サイドバー `P2P ネットワーク` カードの `再取得` ボタンが `isRefreshingStatus` 中に `更新中…` へ変化し、最終更新/次回再取得表示が更新されることを確認。
- 本 Runbook 内の監視手順（メトリクスダッシュボード、手動 `connect_to_peer`）を実施する際は、`connection_status` が `disconnected`→`connected` に遷移するタイミング、`peers` セクションに Node ID が表示されることを確認する。UI の詳細は `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` 5.5 節を参照。

## 10. ブートストラップ CLI / UI 連携（2025年11月09日追加）

### 10.1 `kukuri-cli` ブートストラップノード手順
1. ビルド: `cd kukuri-cli && cargo build --release`。Docker を使う場合は `docker compose up -d bootstrap-node-1 bootstrap-node-2` もしくは `./scripts/start-bootstrap-nodes.ps1 -Mode bootstrap` を実行する。
   - PoC では CLI が書き出す JSON の保管場所を明示する。`--export-path` 未指定時は下表の既定パスへ保存されるため、Tauri アプリと CLI は同じ OS アカウントで実行すること。

     | OS | 既定パス (`dirs::data_dir()/kukuri/cli_bootstrap_nodes.json`) | 備考 |
     | --- | --- | --- |
     | Windows | `%LocalAppData%\kukuri\cli_bootstrap_nodes.json` | 例: `C:\Users\<User>\AppData\Local\kukuri\cli_bootstrap_nodes.json`。PowerShell では `$env:LOCALAPPDATA\\kukuri\\cli_bootstrap_nodes.json`。 |
     | macOS | `$HOME/Library/Application Support/kukuri/cli_bootstrap_nodes.json` | `KUKURI_CLI_BOOTSTRAP_PATH` で上書き可。 |
     | Linux | `$XDG_DATA_HOME/kukuri/cli_bootstrap_nodes.json`（未定義時 `$HOME/.local/share/kukuri/cli_bootstrap_nodes.json`） | CI/Nightly は `KUKURI_CLI_BOOTSTRAP_PATH` を明示し、成果物パスを `phase5_ci_path_audit.md` に記録する。 |
2. 単体ノード起動サンプル:
   ```bash
   RUST_LOG=info ./target/release/kukuri-cli bootstrap \
     --bind 0.0.0.0:11223 \
     --peers k51qzi5uqu5dl@127.0.0.1:44001,k51qzi5uqu5dn@127.0.0.1:44002
   ```
   - `--bind` で待受ポートを指定。`--peers` は既存ブートストラップノードの `node_id@host:port` 形式。
   - 環境変数 `BIND_ADDRESS` / `LOG_LEVEL` / `JSON_LOGS` でも同値を指定できる。
3. 起動ログには `Node ID:` が出力される。接続先クライアントは `KUKURI_BOOTSTRAP_PEERS="node_id@host:port,...`"` に追加し、`pnpm tauri dev` などアプリ起動前に環境変数を読み込む。Docker テスト (`./scripts/test-docker.sh --bootstrap <peers>`) でも同じ書式を利用する。
4. ブートストラップノードのヘルスチェック:
   - Bash: `./scripts/test-docker.sh p2p --bootstrap <node_id@host:port>` → 自動で `p2p-bootstrap` コンテナを起動し、正常終了後に停止。
   - PowerShell: `./scripts/test-docker.ps1 rust -Bootstrap "<node_id@host:port>"`。
5. CLI 実装の回帰テストは `cargo test --package kukuri-cli -- test_bootstrap_runbook` を新設。Runbook に従った設定値（bind/peers/env優先順位）が崩れていないかを CI で検証する。

### 10.2 Settings / RelayStatus との連携
- サイドバーの `RelayStatus` カードに `Runbook` リンクを追加し、本ドキュメント（GitHub: `docs/03_implementation/p2p_mainline_runbook.md`）を即座に開けるようにした。`再試行` ボタンと自動バックオフ更新は `refreshRelaySnapshot`（`src/components/RelayStatus.tsx`）で `useAuthStore.updateRelayStatus` と `p2pApi.getBootstrapConfig` を同時実行するため、CLI が `cli_bootstrap_nodes.json` を更新した直後でも UI が再取得できる。テスト: `pnpm vitest src/tests/unit/components/RelayStatus.test.tsx`。
- 設定画面 > ネットワーク > ブートストラップでは、現在の `KUKURI_BOOTSTRAP_PEERS` とソース（環境変数/アプリ設定/バンドルデフォルト）を表示。環境変数でロックされている場合は UI から編集できない旨を Runbook に追記。
- ブートストラップリストを UI から更新した場合は `app.conf` に追記し、次回起動時に `ENABLE_P2P_INTEGRATION=1` で自動的に読み込む。CI では `scripts/test-docker.{sh,ps1}` が Runbook 記載の値を設定しているため、Runbook 更新後は必ずスクリプトに同じパラメータを反映する。

### 10.3 `kukuri-cli` ブートストラップリスト動的更新 PoC（2025年11月11日追加）
1. `kukuri-cli bootstrap --export-path <path>` でノードを起動すると、`node_id@bind_addr` と指定済み `--peers` が JSON (`{"nodes":[...],"updated_at_ms":...}`) に書き出される。`--export-path` を省略した場合は `KUKURI_CLI_BOOTSTRAP_PATH` 環境変数、未設定時は既定の `%LocalAppData%\kukuri\cli_bootstrap_nodes.json`（macOS/Linux: `$XDG_DATA_HOME/kukuri/cli_bootstrap_nodes.json`）へ保存される。
2. Tauri アプリ起動時に `bootstrap_config::load_cli_bootstrap_nodes` が同ファイルを検出すると、`RelayStatus` カード下部に「CLI 提供: n件 / 更新: ○分前」「最新リストを適用」ボタンが表示される。`KUKURI_BOOTSTRAP_PEERS` でロックされている場合はボタンが無効化され、環境変数を解除しない限り適用できない仕様。
3. `最新リストを適用` を押下すると `apply_cli_bootstrap_nodes` コマンドが `user_bootstrap_nodes.json` に CLI リストをコピーし、続けて `NetworkService::apply_bootstrap_nodes` を呼び出して Mainline DHT へ即時接続する。UI は現在のソース（env/user/bundle/fallback/none）と CLI リストの更新時刻を並列表記するため、Runbook の Chapter2/Chapter6 で参照するブートストラップログと整合が取れる。アプリの再起動は不要。
4. PoC 検証手順:
    - `kukuri-cli bootstrap --export-path "%LocalAppData%\kukuri\cli_bootstrap_nodes.json" --peers node_id@host:port` を実行して JSON が作成されることを確認。
    - `pnpm tauri dev` → サイドバー下部の `RelayStatus` で CLI リストが検知され、「最新リストを適用」押下後に `kukuri-tauri` ログへ `Connected to bootstrap peer from config:` が即座に出力されることを確認（`NetworkService::apply_bootstrap_nodes` が走った証跡）。`get_relay_status` の再取得でステータスが `connected` へ遷移するかを合わせて確認する。
    - 取得ログとスクリーンショットは `tmp/logs/relay_status_cli_bootstrap_<timestamp>.log`（例: PowerShell で `Start-Transcript -Path tmp/logs/relay_status_cli_bootstrap_20251112-094500.log` 実行後に `pnpm tauri dev` を起動）へ保存し、`phase5_ci_path_audit.md` の Nightly セクションにパスを追記する。ログには `cli_nodes_detected`, `apply_cli_bootstrap_nodes`, `Connected to bootstrap peer` を含める。
5. RelayStatus の `refreshRelaySnapshot` により、手動 `再試行`・自動バックオフ・CLI 適用後の再取得がすべて同じコードパスを通り、`p2pApi.getBootstrapConfig` が毎回呼び出される。別端末で `cli_bootstrap_nodes.json` が更新された場合も、次のバックオフ周期で UI が「CLI 提供」件数と更新時刻を再描画する。

### 10.4 Ops / Nightly 連携
- `phase5_ci_path_audit.md` に CLI ブートストラップ PoC 向けの行を追加し、PowerShell/Bash いずれでも `Start-Transcript` または `script` で `tmp/logs/relay_status_cli_bootstrap_<timestamp>.log` を保存する。サンプルログは `tmp/logs/relay_status_cli_bootstrap_20251112-094500.log` を参照。
- Nightly Frontend Unit Tests には `pnpm vitest src/tests/unit/components/RelayStatus.test.tsx` を常時含め、`cargo test --package kukuri-cli -- test_bootstrap_runbook` を `nightly.yml` の `native-test-linux` ジョブへ追記する。Runbook 更新時は両テストのログ ID を `tasks/status/in_progress.md` と `docs/01_project/roadmap.md` Ops 行に必ず転記する。
- Ops チームは Runbook の Chapter2（前提変数）と Chapter10（CLI PoC）をセットで参照し、`KUKURI_BOOTSTRAP_PEERS` によるロックが掛かっていないかを `RelayStatus` UI の `ブートストラップソース` 表示で確認してから適用する。適用後は `p2p_metrics_export --job p2p` で Mainline 接続数の増加を確認し、失敗時は Chapter7 のトラブルシューティング手順に従って再取得する。
   - `pnpm vitest src/tests/unit/components/RelayStatus.test.tsx` / `cargo test --package kukuri-cli -- test_bootstrap_runbook` / `cargo test --package kukuri-tauri --test test_event_service_gateway` を実行し、UI・CLI・Gateway の回帰テストが通ることを Runbook に記録する。
