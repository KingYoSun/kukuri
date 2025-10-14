# P2P DHTテスト戦略

最終更新日: 2025年10月14日

## 目的
- kukuri-tauri の P2P 受信確認テストを、iroh の DHT ディスカバリーのみで安定実行する。
- テスト用にローカル生成エンドポイント同士を手動接続せず、ブートストラップノード経由でメッシュを構築する。

## 背景
- 既存の `iroh_integration_tests.rs` は `connect_peers` で直接接続する設計だったが、接続タイミングとイベント受信の同期が難しく不安定だった。
- 当面は Nostr リレーに依存せず、iroh DHT + Gossip のみで体験を完結させる方針（`docs/01_project/activeContext/tasks/status/in_progress.md` 参照）。

## 対応方針
1. Docker テスト環境に `kukuri-cli` の `bootstrap` サービスを常駐させ、全テストノードのディスカバリーフックとする。
2. Rust 統合テストでは `Endpoint::builder().discovery_dht()` のみを利用し、`connect_peers` を廃止する。
3. `scripts/test-docker.{ps1,sh}` がブートストラップノードを起動し、`KUKURI_BOOTSTRAP_PEERS` を未指定の場合は既定値（`03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233`）へ上書きする。
4. トピック参加後は DHT 経由でピアが可視化されるまで待機するヘルパーを用意し、`PeerJoined` イベントや `get_joined_topics()` を監視してタイムアウトする仕組みを整える。

## 変更案概要
- `docker-compose.test.yml`
  - `p2p-bootstrap` サービスを追加し、`kukuri-cli` イメージを再利用した DHT ブートストラップノードを常時起動。
  - `network_mode: host` で 11233/TCP をリッスン、`KUKURI_SECRET_KEY` を固定化（NodeId は `03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8`）。
  - `netcat-openbsd` を導入した上で `healthcheck`（`nc -z 127.0.0.1 11233`）を設定し、テスト実行前にヘルス確認が行えるようにした。
- `scripts/test-docker.sh` / `.ps1`
  - `integration` 実行時に `p2p-bootstrap` を起動し、ヘルスチェック完了まで待機。終了後は必ず `down --remove-orphans` でクリーンアップ。
  - PowerShell 版はホスト環境変数を一時的に設定し、Unix 版は `kukuri-tauri/tests/.env.p2p` を生成することで `ENABLE_P2P_INTEGRATION=1` / `KUKURI_FORCE_LOCALHOST_ADDRS=0` / `KUKURI_BOOTSTRAP_PEERS=...` をコンテナへ注入。
  - テスト実行は `cargo test --package kukuri-tauri --lib modules::p2p::tests::iroh_integration_tests:: -- --nocapture --test-threads=1` に絞り込み、既存のユニットテストを巻き込まずに P2P 経路の結合テストのみを実行する。
  - 失敗時にもブートストラップを停止できるようコマンド失敗を捕捉し、後続のクリーンアップとエラーハンドリングを分離。
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
