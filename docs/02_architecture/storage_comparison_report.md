# データストレージ設計レビュー：iroh-docs vs sqlx

## 調査日時：2025年07月25日

## エグゼクティブサマリー

kukuriプロジェクトのデータストレージとして、**sqlx（SQLite）を推奨**します。iroh-docsは分散同期に優れていますが、Nostrプロトコルとの統合において、sqlxの方が柔軟性と実装の容易さで優位性があります。

## 1. 技術比較表

| 比較項目 | iroh-docs | sqlx (SQLite) |
|---------|-----------|--------------|
| **データモデル** | キー/値ストア（BLAKE3ハッシュ） | リレーショナルDB |
| **同期方式** | 自動（レンジベース集合調整） | 手動実装が必要 |
| **Nostrイベント保存** | 要変換（ハッシュのみ保存） | 直接保存可能 |
| **クエリ能力** | 限定的（キーベース） | 完全なSQL |
| **インデックス** | なし | 柔軟に定義可能 |
| **トランザクション** | なし | ACID準拠 |
| **コンパイル時チェック** | なし | query!マクロで可能 |
| **パフォーマンス** | P2P同期に最適化 | ローカル読み書きに最適化 |

## 2. Nostr互換性分析

### 2.1 sqlxの優位性

1. **Nostrイベント構造の直接保存**
   ```rust
   // sqlxでの実装例
   sqlx::query!(
       "INSERT INTO events (id, pubkey, created_at, kind, tags, content, sig, topic_id) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
       event.id, event.pubkey, event.created_at, event.kind, 
       serde_json::to_string(&event.tags)?, event.content, event.sig, topic_id
   )
   .execute(&pool)
   .await?;
   ```

2. **柔軟なクエリ**
   - タイムライン取得、フィルタリング、検索が容易
   - Nostr NIPs準拠のフィルタ実装が簡単

3. **既存エコシステムとの統合**
   - Nostrリレーとの互換性維持が容易
   - 標準的なNostrクライアントツールの使用可能

### 2.2 iroh-docsの課題

1. **データ変換オーバーヘッド**
   - Nostrイベント → BLAKE3ハッシュへの変換必要
   - コンテンツ取得に追加ステップ（ハッシュ→blob取得）

2. **クエリ制限**
   - タイムスタンプベースの範囲クエリが困難
   - 複雑なフィルタリング（tags、kind）の実装が複雑

3. **既存ツールとの非互換性**
   - Nostr標準ツールが使用不可
   - カスタムブリッジ層の開発必要

## 3. kukuriプロジェクト要件との適合性

### 3.1 必須要件の評価

| 要件 | iroh-docs | sqlx | 判定 |
|------|-----------|------|------|
| Nostrイベント保存 | △要変換 | ○直接可能 | sqlx優位 |
| 高速タイムライン | △追加実装要 | ○SQLで簡単 | sqlx優位 |
| オフライン対応 | ○自動同期 | △手動実装 | iroh-docs優位 |
| 署名検証 | ー別途実装 | ー別途実装 | 同等 |
| インデックス検索 | ×不可 | ○完全対応 | sqlx優位 |

### 3.2 パフォーマンス要件

- **起動時間（3秒以内）**: 両方とも達成可能
- **タイムライン読み込み（1秒以内）**: sqlxが有利（最適化されたクエリ）
- **スケーラビリティ**: sqlxで十分（10万ユーザー対応可能）

## 4. 推奨アーキテクチャ

### 4.1 ハイブリッドアプローチ（将来的オプション）

```rust
// メインストレージ：sqlx
// P2P同期：iroh（blobsのみ）
// 大容量メディア：iroh-blobs

struct HybridStorage {
    sql: SqlitePool,           // Nostrイベント、メタデータ
    blobs: iroh_blobs::Store,  // 画像、動画
}
```

### 4.2 当面の実装方針（MVP）

1. **sqlxベースの実装**
   - Nostrイベントの完全な保存
   - 高速なクエリとインデックス
   - トランザクション保証

2. **P2P同期は手動実装**
   - Nostrプロトコル準拠の同期
   - 差分同期アルゴリズム
   - WebSocketベースの通信

## 5. 実装推奨事項

### 5.1 即時実装

```rust
// 1. SQLiteスキーマの最適化
CREATE INDEX idx_events_created_at_desc ON events(created_at DESC);
CREATE INDEX idx_events_kind_topic ON events(kind, topic_id);
CREATE INDEX idx_events_tags ON events(json_extract(tags, '$'));

// 2. 接続プール設定
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(
        SqliteConnectOptions::from_str(&db_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
    )
    .await?;
```

### 5.2 将来的な拡張

1. **Phase 2でiroh-blobsを追加**
   - メディアファイルのP2P配信
   - 帯域幅の削減

2. **カスタム同期レイヤー**
   - Nostrイベントの効率的な同期
   - 既読管理とキャッシュ戦略

## 6. リスクと緩和策

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| SQLiteのスケーラビリティ限界 | 中 | 分散DB移行パスを設計 |
| P2P同期の複雑性 | 高 | 段階的実装とテスト |
| ファイルサイズ増大 | 中 | 定期的なVACUUMとアーカイブ |

## 7. 結論

kukuriプロジェクトのMVPフェーズでは、**sqlx（SQLite）**の採用を推奨します。理由：

1. **Nostrプロトコルとの親和性が高い**
2. **実装がシンプルで保守性が高い**
3. **必要十分な性能を提供**
4. **将来的な拡張が容易**

iroh-docsは優れた分散同期機能を持ちますが、Nostrエコシステムとの統合において追加の複雑性をもたらします。Phase 2以降で、メディアファイルのP2P配信にiroh-blobsを部分的に採用することを検討してください。