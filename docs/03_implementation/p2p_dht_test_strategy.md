# P2P DHTテスト戦略

最終更新日: 2025年10月15日

## 目的
- kukuri-tauri の P2P 受信確認テストを、iroh の DHT ディスカバリーのみで安定実行する。
- テスト用にローカル生成エンドポイント同士を手動接続せず、ブートストラップノード経由でメッシュを構築する。

## 背景
- 既存の `iroh_integration_tests.rs` は `connect_peers` で直接接続する設計だったが、接続タイミングとイベント受信の同期が難しく不安定だった。
- 当面は Nostr リレーに依存せず、iroh DHT + Gossip のみで体験を完結させる方針（`docs/01_project/activeContext/tasks/status/in_progress.md` 参照）。

## 対応方針
1. `docker compose -f docker-compose.test.yml up --build --exit-code-from test-runner p2p-bootstrap test-runner` を実行し、Rust P2P 統合テストと TypeScript 統合テストによる最小スモークを確認。
2. 必要に応じて `./scripts/test-docker.sh build` でテスト用イメージを再構築し、キャッシュ汚染がない状態を作る。
3. Windows/Linux 共通で `./scripts/test-docker.ps1 integration` または `./scripts/test-docker.sh integration` を用い、P2P 統合テストのみを実行（ブートストラップが自動起動し、`ENABLE_P2P_INTEGRATION=1` が付与される）。
4. 通常の Rust テストは `./scripts/test-docker.ps1 rust` または `./scripts/test-docker.sh rust` で実行し、`ENABLE_P2P_INTEGRATION=0` のまま高速に完了することを確認。
5. 失敗した場合は `docker logs kukuri-p2p-bootstrap` や `docker inspect kukuri-p2p-bootstrap --format '{{.State.Health}}'` を確認し、NodeId や Discovery 初期化に問題がないか調査する。
## 変更案概要
- `docker-compose.test.yml`
  - `p2p-bootstrap` サービスを追加し、`kukuri-cli` イメージを利用した DHT ブートストラップノードを常時起動。
  - `network_mode: host` で 11233/TCP を露出し、`KUKURI_SECRET_KEY` を固定化して決定論的な NodeId (`03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8`) を生成。
  - `test-runner` サービスの既定コマンドを `run-smoke-tests.sh` に変更し、Rust P2P 統合テスト (`cargo test --package kukuri-tauri --lib modules::p2p::tests::iroh_integration_tests:: -- --nocapture --test-threads=1`) と TypeScript 統合テスト (`pnpm test:integration`) のみを実行してスモークを最小化。
  - `p2p-bootstrap` の `healthcheck` 成功を待ってから開始するよう `depends_on` を追加し、`docker compose up` 実行時にブートストラップ起動待ちが保証されるようにした。
  - `ENABLE_P2P_INTEGRATION=1` / `KUKURI_FORCE_LOCALHOST_ADDRS=0` / `KUKURI_BOOTSTRAP_HOST` / `KUKURI_BOOTSTRAP_PORT` を既定で埋め込み、`BOOTSTRAP_WAIT_SECONDS` で待機秒数を調整できるようにした。
- `scripts/test-docker.sh` / `.ps1`
  - `integration` 実行時に `p2p-bootstrap` を起動し、ヘルスチェック完了まで待機。終了後は `down --remove-orphans` でクリーンアップ。
  - PowerShell 版はホスト側環境変数を一時的に設定し、Unix 版は `kukuri-tauri/tests/.env.p2p` を生成して `ENABLE_P2P_INTEGRATION=1` / `KUKURI_FORCE_LOCALHOST_ADDRS=0` / `KUKURI_BOOTSTRAP_PEERS=...` を注入。
  - `all` / `rust` / `ts` / `lint` コマンドは `docker-compose.test.yml` の `test-runner` に対して `/app/run-tests.sh` を明示的に実行し、従来のフルスイートを維持する。
  - スモーク用途ではコンテナ既定の `run-smoke-tests.sh` をそのまま利用し、`docker compose up --build p2p-bootstrap test-runner` で Tauri 非依存の検証が完結する。
  - 失敗時にもブートストラップを停止できるよう例外処理を維持し、クリーンアップとエラーハンドリングを分離。
- `iroh_integration_tests.rs`
  - DHT ブートストラップ必須の `bootstrap_context()` を追加し、`ENABLE_P2P_INTEGRATION!=1` や `KUKURI_BOOTSTRAP_PEERS` 未設定時はテストをスキップ。
  - `create_service_with_endpoint` / `connect_peers` を廃止し、`Endpoint::builder().discovery_dht()` + `endpoint.connect(bootstrap)` で初期化する `create_service()` を導入。
  - `wait_for_peer_join_event` ヘルパーで `PeerJoined` イベントを待機し、テストごとにブートストラップヒントを `join_topic` へ配布して DHT への参加完了を確認。
  - タイムアウトを 15 秒前後へ延長し、受信失敗時にはウォーンログを出力することでデバッグ容易性を向上。
  - `build_peer_hints` で各ノードのローカル `node_id@host:port` ヒントを共有し、ブートストラップ前でも接続を確立できるようにする。
  - `IrohGossipService::local_peer_hint()` と `join_topic` のタイムアウト制御を組み合わせ、`receiver.joined()` が未完了でもテストが完走するよう調整。
- ドキュメント
  - 本ドキュメントおよび `docker_test_environment.md` を更新し、`.ps1` / `.sh` の自動化内容と既定値の扱いを明文化。

## 検証ステップ
1. `./scripts/test-docker.sh build` で新イメージを作成。
2. Windows/Linux 共通で `./scripts/test-docker.ps1 integration` または `./scripts/test-docker.sh integration` を用い、P2P 統合テストのみを実行（ブートストラップが自動起動し、`ENABLE_P2P_INTEGRATION=1` が付与される）。
3. 通常の Rust テストは `./scripts/test-docker.ps1 rust`（または `./scripts/test-docker.sh rust`）で実行し、`ENABLE_P2P_INTEGRATION=0` のまま高速に完了することを確認。
4. 失敗した場合は `docker logs kukuri-p2p-bootstrap` や `docker inspect kukuri-p2p-bootstrap --format '{{.State.Health}}'` を確認し、NodeId や Discovery 初期化に問題がないか調査する。

## 今後のフォロー
- テスト用トピック一覧を集約管理する仕組み（例: `tests/p2p_topics.rs`）を整備する。
- `p2p-bootstrap` のトピック購読数が増えた場合に備え、将来的な設定ファイル化や `--topics-file` オプション追加を検討する。
- DHT メトリクスをテスト中に取得し、受信遅延の傾向を観測できるようにする。
- TypeScript 契約テストを追加し、P2P 経路の返信・引用ケースを E2E 以外でも担保する。
- Windows 向け `scripts/test-docker.ps1` に `metrics` / `contracts` オプションを追加し、Docker 経由でメトリクス取得と契約テストを実行できるようにする。
