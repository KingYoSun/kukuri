# P2P Mainline Runbook
最終更新日: 2025年10月22日

## 1. 目的
- Mainline DHT を有効にした P2P ネットワークの運用手順と統合テスト実行フローを共有する。
- 開発環境と CI の両方で同一手順を踏めるよう、必要な環境変数・ログ設定・トラブルシューティングを整理する。

## 2. 前提条件
- `ENABLE_P2P_INTEGRATION=1` を設定し、統合テストを明示的に許可する。
- `KUKURI_BOOTSTRAP_PEERS="node_id@host:port,...`"` で有効なブートストラップノードを指定する。
- 必要に応じて `KUKURI_IROH_BIN` で利用する `iroh` バイナリのパスを上書きできる（未設定時は PATH を参照）。
- Windows 環境では PowerShell 用スクリプト `./scripts/start-bootstrap-nodes.ps1` を用い、テスト前にノード群を起動する。

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
  - `scripts/docker/run-smoke-tests.sh` / `scripts/test-docker.{sh,ps1}` は両テストを順次実行する構成に統一済みで、旧バイナリへのフォールバックは存在しない。

### 3.1 共通ユーティリティの活用
- Phase 4 でテスト支援コードを `application/shared/tests/p2p` へ集約済み。新しい `p2p_*_smoke` からも同ユーティリティを利用する。  
- Gossip 経路で利用する DefaultTopicsRegistry や EventPublisher も `application/shared` へ移動済み。自前で複製せず共有モジュールを参照して重複を避ける。  
- 新しいシナリオを追加する場合は `src-tauri/tests/common` を再利用し、smoke テストに倣って `ENABLE_P2P_INTEGRATION` とブートストラップ待機処理を組み込む。

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

## 7. トラブルシューティング
- **`STATUS_ENTRYPOINT_NOT_FOUND`**: Windows で iroh バイナリの依存 DLL が見つからない場合に発生。`KUKURI_IROH_BIN` を明示し、`PATH` に `libssl` 等が含まれているか確認。Docker 実行で迂回可能。
- **ブートストラップ接続失敗**: `KUKURI_BOOTSTRAP_PEERS` の NodeId/ポートを再確認し、ファイアウォールで該当ポートを開放する。
- **テストタイムアウト**: `support/bootstrap.rs` の `DEFAULT_JOIN_TIMEOUT` / `DEFAULT_EVENT_TIMEOUT` を一時的に延長し、`RUST_LOG=debug` でどこまで進んでいるかを追跡。根治策はブートストラップノードのキャパシティ調整。
- **ログが出力されない**: 既に別のサブスクライバが設定されている可能性。テスト起動前に `RUST_LOG` を設定し、`tracing_subscriber::fmt` 初期化が一度だけ呼ばれているかを確認。

## 8. 今後の TODO
- 再接続・再索引シナリオ用の `recovery.rs` を実装し、OfflineService との結合テストを追加。
- iroh バイナリのキャッシュ戦略（GitHub Actions 用）を整備し、ダウンロード時間を短縮する。
