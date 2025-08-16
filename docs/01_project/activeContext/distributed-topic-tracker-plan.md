# [DEPRECATED] Kukuri: Distributed-Topic-Tracker を用いたブートストラップ機能の設計ドキュメント

> **⚠️ 注意: このドキュメントは廃止されました**
> 
> **更新日: 2025年08月16日**
> 
> irohがビルトインでBitTorrent Mainline DHTをサポートしていることが判明したため、
> distributed-topic-trackerの使用は不要となりました。
> 
> **新しい計画書を参照してください: [iroh-native-dht-plan.md](./iroh-native-dht-plan.md)**

---

## 1. 導入（旧内容）

### 1.1 背景
kukuri は、Nostr プロトコルに基づく分散型 P2P ソーシャルアプリケーションで、iroh と iroh-gossip を活用したイベント配信とエンドツーエンド暗号化を特徴としています。現在の開発では、Cloudflare Workers を用いた中央集権型ブートストラップを想定していましたが、まだ未実装のため、分散型アプローチへの移行が容易です。

[distributed-topic-tracker](https://github.com/rustonbsd/distributed-topic-tracker) （API Doc: https://docs.rs/crate/distributed-topic-tracker/latest/json）は、BitTorrent Mainline DHT を基盤とした分散型トピックトラッカーで、iroh-gossip のブートストラップをサーバーレスで実現します。これを kukuri に統合することで、真の分散性を達成し、検閲耐性とスケーラビリティを向上させることができます。

### 1.2 目的
- iroh-gossip のブートストラップを分散型に置き換え、中央サーバー依存を排除。
- ノード発見を自動化し、共有シークレットと Ed25519 署名でセキュアにピア接続を確立。
- kukuri の P2P 通信 (src-tauri/src/p2p/) にシームレスに統合し、トピックベースのタイムライン同期を効率化。

### 1.3 範囲
- 対象: Rust バックエンド (Tauri v2)。
- 統合ポイント: P2P 初期化部分 (e.g., iroh エンドポイントのセットアップ)。
- 非対象: フロントエンド (React) の UI 変更は最小限 (e.g., ステータス表示のみ)。

## 2. 要件

### 2.1 機能要件
- **ブートストラップの自動化**: DHT を用いてトピック ID と共有シークレットでピアを発見。リトライとジッターで耐故障性を確保。
- **セキュアな発見**: Ed25519 署名と共有シークレットの回転で、Sybil 攻撃を防ぐ。
- **レート制限**: DHT レコードの公開/取得を分単位で制限し、過負荷を回避。
- **イベントハンドリング**: 新規ピア接続、メッセージ受信、ピアマージを処理。
- **フォールバック**: DHT 失敗時、既知ピアリスト (ハードコードまたはローカルストレージ) を使用。
- **互換性**: 既存の iroh-gossip インスタンスと統合。Nostr イベントのトピック (e.g., タイムライン ID) をサポート。

### 2.2 非機能要件
- **パフォーマンス**: 初回ブートストラップを 5-10 秒以内に完了 (DHT 検索遅延考慮)。
- **スケーラビリティ**: 数千ノード規模で動作。背景パブリッシャーで定期再公開。
- **セキュリティ**: 共有シークレットをセキュアストレージ (keyring) に保存。
- **テスト**: ユニットテスト (cargo test) と E2E テスト (Docker Compose で複数ノードシミュレーション)。
- **依存関係**: Rust 1.70+, iroh, iroh-gossip, distributed-topic-tracker (v0.1.1)。

### 2.3 制約
- インターネットアクセスなしのオフライン動作はサポートせず (DHT 依存)。
- モバイル対応時は、バッテリー/ネットワーク消費を考慮 (将来の最適化)。

## 3. アーキテクチャ

### 3.1 全体概要
kukuri のバックエンドは Tauri で Rust を使用。distributed-topic-tracker を iroh-gossip に統合し、以下のように動作:

1. **初期化**: iroh エンドポイントを作成し、AutoDiscoveryBuilder で DHT ベースのブートストラップを設定。
2. **トピック参加**: ユーザーがトピック (Nostr イベントのグループ) に参加時、共有シークレットで DHT に登録/検索。
3. **ピア発見**: DHT からピアリストを取得し、gossip 接続を確立。
4. **イベント配信**: iroh-gossip でメッセージをブロードキャスト/受信。
5. **背景処理**: 定期的にピアを再公開し、バブル (孤立クラスタ) を検知/マージ。

図 (テキストベース):

```
[Frontend (React/Zustand)] --> [Tauri Commands] --> [Rust Backend]
                                      |
                                      v
[DB (SQLite)] <--> [P2P Module] <--> [iroh Endpoint]
                        |
                        v
[distributed-topic-tracker] <--> [BitTorrent DHT]
                        |
                        v
[AutoDiscoveryGossip] --> [iroh-gossip Swarm]
```

### 3.2 コンポーネント
- **AutoDiscoveryBuilder**: DHT 設定 (レート制限、リトライ回数、ジッター)。
- **AutoDiscoveryGossip**: gossip インスタンスのラッパー。イベント (メッセージ、ピア接続) をハンドル。
- **TopicId & Shared Secret**: Nostr トピックを TopicId にマップ。シークレットは回転 (e.g., 每日)。
- **Integration Point**: src-tauri/src/p2p/init.rs (新規作成) でエンドポイントをセットアップ。

### 3.3 データフロー
1. アプリ起動 → P2P 初期化 → DHT ブートストラップ。
2. トピック参加 → DHT 検索 → ピア取得 → gossip 参加。
3. イベント受信 → Zustand ストア更新 → UI 反映。

## 4. 実装計画

### 4.1 ステップバイステップ
1. **依存追加**: Cargo.toml に以下を追加。
   ```
   [dependencies]
   distributed-topic-tracker = "0.1.1"
   iroh = "*"  # 既存
   iroh-gossip = "*"  # 既存
   anyhow = "1"
   tokio = "1"
   ```

2. **P2P モジュール更新**: src-tauri/src/p2p/ に bootstrap.rs を作成。
   - iroh エンドポイントを作成。
   - AutoDiscoveryBuilder でビルド (レート制限: 10 records/min, リトライ: 3)。

3. **Tauri コマンド統合**: src-tauri/src/commands/p2p.rs でブートストラップコマンドを追加。
   - e.g., `#[tauri::command] async fn init_p2p() -> Result<()>`

4. **イベントハンドリング**: gossip イベントループでメッセージを処理し、SQLite に保存。

5. **共有シークレット管理**: keyring で保存/回転。DefaultSecretRotation を使用。

6. **フォールバック実装**: DHT 失敗時、ハードコードされたブートストラップノードを使用 (将来廃止)。

### 4.2 コード例
最小統合例 (bootstrap.rs ベース):

```rust
use anyhow::Result;
use distributed_topic_tracker::{AutoDiscoveryBuilder, AutoDiscoveryGossip, DefaultSecretRotation, TopicId};
use iroh::net::Endpoint;
use iroh_gossip::{Gossip, GossipEvent};
use tokio::sync::mpsc;

#[tokio::main]
async fn init_bootstrap(shared_secret: &[u8; 32]) -> Result<()> {
    let endpoint = Endpoint::builder().bind(0).await?;  // iroh エンドポイント

    let builder = AutoDiscoveryBuilder::default()
        .rate_limit(10)  // 分あたりレコード制限
        .retries(3)      // リトライ回数
        .jitter(500);    // ジッター (ms)

    let (gossip_tx, gossip_rx) = mpsc::channel(32);
    let gossip = AutoDiscoveryGossip::spawn(endpoint.clone(), builder, gossip_tx).await?;

    // トピック参加 (Nostr トピックを TopicId に変換)
    let topic_id = TopicId::from_bytes(b"example-topic");
    let secret_rotation = DefaultSecretRotation::new(shared_secret);
    gossip.join(topic_id, secret_rotation).await?;

    // イベント処理ループ
    while let Some(event) = gossip_rx.recv().await {
        match event {
            GossipEvent::Message { from, content } => {
                // Nostr イベント処理 (SQLite 保存)
                println!("Received: {:?}", content);
            }
            GossipEvent::PeerConnected(peer) => {
                println!("New peer: {:?}", peer);
            }
            _ => {}
        }
    }

    Ok(())
}
```

- Tauri コマンドから呼び出し: `init_bootstrap(&get_shared_secret()?).await?`

## 5. テストとデバッグ

### 5.1 ユニットテスト
- cargo test で AutoDiscoveryBuilder の設定を検証。
- モック DHT でピア発見をシミュレート。

### 5.2 E2E テスト
- Docker Compose で複数コンテナ起動 (./test-e2e.sh 参考)。
- シナリオ: ノード A がトピック参加 → ノード B が発見/接続 → メッセージ交換確認。

### 5.3 デバッグ
- ログ: tracing crate で DHT クエリ/イベントをログ。
- メトリクス: Prometheus 統合でピア数/レイテンシを監視 (オプション)。

## 6. 潜在的な課題と解決策

- **DHT 遅延**: リトライ/ジッターで緩和。UI でプログレス表示。
- **プライバシー漏洩**: アドレス公開を最小化 (DHT の性質上避けられないが、署名で認証)。
- **Sybil 攻撃**: 共有シークレット回転とレート制限で防ぐ。
- **移行コスト**: Cloudflare 想定を削除し、テストで検証。初期はハイブリッド (DHT + フォールバック)。
- **リソース消費**: 背景タスクを低頻度に (e.g., 5 分毎再公開)。

## 7. ロードマップ
- Phase 1: プロトタイプ統合 (1-2 週間)。
- Phase 2: E2E テストと最適化 (1 週間)。
- Phase 3: 本番リリースとドキュメント更新 (README に追加)。

## 8. 結論
この設計により、kukuri は完全分散型のブートストラップを実現し、P2P ソーシャルアプリとしての競争力を高めます。distributed-topic-tracker の成熟度 (テスト中) を考慮し、定期アップデートを監視。質問やフィードバックは GitHub Issue で。
