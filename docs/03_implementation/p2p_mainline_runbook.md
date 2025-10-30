# P2P Mainline Runbook
?????: 2025?10?30?

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
- Phase 5 で `p2p_metrics_export` バイナリとラッパースクリプト `scripts/metrics/export-p2p.{sh,ps1}` を追加した。CI ではテスト後に下記コマンドで `docs/01_project/activeContext/artefacts/metrics/<timestamp>-p2p-metrics.json` を生成し、成果物として保存すること。
  ```bash
  ./scripts/metrics/export-p2p.sh --pretty
  ```
- PowerShell 版は `./scripts/metrics/export-p2p.ps1 -Pretty` で同じ JSON を出力する。`--output` / `-Output` オプションで保存先を上書き可能。
- エクスポートされた JSON には Gossip/Mainline 双方のカウンタ・直近タイムスタンプが含まれるため、CI で期待件数との差分を検証したり、進捗レポートに添付する。

### 6.3 iroh バイナリキャッシュ
- GitHub Actions では `actions/cache@v4` を利用し、`~/.cache/kukuri/iroh`（PowerShell 版は `%LocalAppData%\kukuri\iroh`）をキャッシュする。キーは `iroh-${{ runner.os }}-${{ hashFiles("scripts/install-iroh.ps1") }}` を推奨し、`scripts/install-iroh.{sh,ps1}` でキャッシュヒット時はダウンロードをスキップする。
- ローカル環境でも `./scripts/install-iroh.ps1 -UseCache`（PowerShell）または `./scripts/install-iroh.sh --use-cache` を使用することで同ディレクトリを再利用し、`docker` テスト前のセットアップ時間を短縮できる。
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
