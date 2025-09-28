# P2P DHTテスト戦略

最終更新日: 2025年09月28日

## 目的
- kukuri-tauri の P2P 受信確認テストを、iroh の DHT ディスカバリーのみで安定実行する。
- テスト用にローカル生成エンドポイント同士を手動接続せず、ブートストラップノード経由でメッシュを構築する。

## 背景
- 既存の `iroh_integration_tests.rs` は `connect_peers` で直接接続する設計だったが、接続タイミングとイベント受信の同期が難しく不安定だった。
- 当面は Nostr リレーに依存せず、iroh DHT + Gossip のみで体験を完結させる方針（`docs/01_project/activeContext/tasks/status/in_progress.md` 参照）。

## 対応方針
1. Docker テスト環境に `kukuri-cli` の `bootstrap` サービスを常駐させ、全テストノードのディスカバリーフックとする。
2. Rust 統合テストでは `Endpoint::builder().discovery_dht()` のみを利用し、`connect_peers` を廃止する。
3. テスト開始時に `KUKURI_BOOTSTRAP_PEERS` を必須化し、指定がない場合は失敗として扱う。
4. トピック参加後は DHT 経由でピアが可視化されるまで待機するヘルパーを用意し、`PeerJoined` イベントや `get_joined_topics()` を監視してタイムアウトする仕組みを整える。

## 変更案概要
- `docker-compose.test.yml`
  - 新サービス `p2p-bootstrap` を追加。
  - `build: ./kukuri-cli`, `command: ["bootstrap"]`, `network_mode: host`, `BIND_ADDRESS=0.0.0.0:11233`。
  - `KUKURI_SECRET_KEY` を固定化し、NodeId `03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8` を共有する。
  - `healthcheck` で TCP ポート 11233 の疎通を確認。
  - `test-runner` / `rust-test` を `depends_on` で待機させる。
- `scripts/test-docker.sh` / `.ps1`
  - テスト前に `docker compose up -d p2p-bootstrap`、終了後に `down p2p-bootstrap` を実行。
  - `.env.p2p` に `KUKURI_BOOTSTRAP_PEERS=<node_id>@127.0.0.1:11233` を書き込むロジックを追加。
- `iroh_integration_tests.rs`
  - `create_service_with_endpoint` を DHT 専用ビルダーへ変更。
  - `connect_peers` ヘルパーを削除し、DHT の近接検知を待つ `wait_for_dht_peer` を実装。
  - 各テスト冒頭で `require_bootstrap()`（未設定時 panic）を呼び出す。
  - 受信確認は `tokio::time::timeout` で最大待機し、失敗時は DHT 参加状況とブートストラップ情報をログ化。
- ドキュメント
  - 本ドキュメントと `docker_test_environment.md` の更新で運用手順を周知。

## 検証ステップ
1. `./scripts/test-docker.sh build` で新イメージを作成。
2. `./scripts/test-docker.sh p2p --tests iroh_integration_tests` を実行し、DHT 経由で受信確認テストが完走することを確認。
3. Windows 環境で `./scripts/test-docker.ps1 rust` を実行し、Docker 経由でも同様に成功することを確認。
4. 失敗した場合は `docker logs kukuri-p2p-bootstrap` を確認し、NodeId や Discovery の初期化に問題がないかを調査。

## 今後のフォロー
- テスト用トピック一覧を集約管理する仕組み（例: `tests/p2p_topics.rs`）を整備する。
- `p2p-bootstrap` のトピック購読数が増えた場合に備え、将来的な設定ファイル化や `--topics-file` オプション追加を検討する。
- DHT メトリクスをテスト中に取得し、受信遅延の傾向を観測できるようにする。
