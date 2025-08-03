# Nostrリアクション機能実装ガイド

**作成日**: 2025年8月3日  
**最終更新**: 2025年8月3日

## 概要

本ドキュメントは、kukuriアプリケーションにおけるNostrプロトコルベースのリアクション機能の実装について説明します。

## 実装されているリアクション機能

### 1. いいね機能（Like）

**実装**:
- コマンド: `like_post`
- Nostrイベント: Kind 7（NIP-25準拠）
- リアクション内容: `"+"`（標準的ないいね）

```rust
// like_postコマンドの実装
state
    .event_manager
    .send_reaction(&event_id, "+")
    .await
    .map_err(|e| format!("いいねに失敗しました: {e}"))?;
```

### 2. ブースト機能（Repost）

**実装**:
- コマンド: `boost_post`
- Nostrイベント: Kind 6（NIP-18準拠）
- 実装: `EventManager::send_repost`

```rust
// boost_postコマンドの実装
state
    .event_manager
    .send_repost(&event_id)
    .await
    .map_err(|e| format!("ブーストに失敗しました: {e}"))?;
```

### 3. カスタムリアクション絵文字

**実装**:
- コンポーネント: `ReactionPicker`
- Nostrイベント: Kind 7（NIP-25準拠）
- リアクション内容: 任意の絵文字（16種類のプリセット）

```typescript
// ReactionPickerで利用可能な絵文字
const POPULAR_REACTIONS = [
  '👍', '❤️', '😄', '😂', '😮', '😢', '😡', '🔥',
  '💯', '🎉', '🚀', '👀', '🤔', '👏', '💪', '🙏',
];
```

### 4. ブックマーク機能

**実装**:
- コマンド: `bookmark_post`, `unbookmark_post`
- 保存先: ローカルSQLiteデータベース
- **注意**: Nostrイベントとしては発行されない（ローカル機能）

## Nostrイベントの構造

### Kind 7: リアクションイベント

```rust
// EventPublisher::create_reaction
let tags = vec![
    Tag::event(*event_id),        // リアクション対象のイベントID
    Tag::public_key(keys.public_key())  // 作成者の公開鍵
];

let event = EventBuilder::new(Kind::Reaction, reaction)
    .tags(tags)
    .sign_with_keys(keys)?;
```

### Kind 6: リポストイベント

```rust
// EventPublisher::create_repost
let tags = vec![
    Tag::event(*event_id),        // リポスト対象のイベントID
    Tag::public_key(keys.public_key())  // 作成者の公開鍵
];

let event = EventBuilder::new(Kind::Repost, "")
    .tags(tags_with_relay)
    .sign_with_keys(keys)?;
```

## データの流れ

### フロントエンド → バックエンド

1. **いいね**: PostCard → TauriApi.likePost → like_post → EventManager.send_reaction
2. **ブースト**: PostCard → TauriApi.boostPost → boost_post → EventManager.send_repost
3. **カスタムリアクション**: ReactionPicker → NostrAPI.sendReaction → send_reaction → EventManager.send_reaction
4. **ブックマーク**: PostCard → bookmarkStore → TauriApi.bookmarkPost → bookmark_post → BookmarkManager

### バックエンド処理

```rust
// EventManager内の処理フロー
1. ensure_initialized() - 初期化確認
2. EventPublisher.create_*() - イベント作成
3. ClientManager.publish_event() - Nostrリレーへ送信
4. EventSync.propagate_nostr_event() - P2Pネットワークへ配信
```

## 状態管理

### フロントエンド

- **いいね数**: Post.likes（楽観的UI更新）
- **ブースト数**: Post.boosts（楽観的UI更新）
- **ブースト状態**: Post.isBoosted
- **ブックマーク状態**: bookmarkStore.bookmarkedPostIds

### バックエンド

- **ブックマーク**: bookmarksテーブル
  ```sql
  CREATE TABLE bookmarks (
      id TEXT PRIMARY KEY,
      user_pubkey TEXT NOT NULL,
      post_id TEXT NOT NULL,
      created_at INTEGER NOT NULL,
      UNIQUE(user_pubkey, post_id)
  );
  ```

## セキュリティ考慮事項

1. **認証**: 全てのリアクション操作で現在のユーザー認証を確認
2. **イベントID検証**: 16進文字列からEventId型への変換時にバリデーション
3. **署名**: 全てのNostrイベントは秘密鍵で署名される

## 今後の拡張可能性

1. **リアクション集計**: 投稿ごとのリアクション数をローカルDBでキャッシュ
2. **カスタムリアクションの保存**: よく使うリアクションの履歴保存
3. **リアクション通知**: 自分の投稿へのリアクションをリアルタイム通知
4. **リアクション分析**: どのようなリアクションが多いかの統計表示

## 参考資料

- [NIP-18: Reposts](https://github.com/nostr-protocol/nips/blob/master/18.md)
- [NIP-25: Reactions](https://github.com/nostr-protocol/nips/blob/master/25.md)
- [nostr-sdk Rust Documentation](https://docs.rs/nostr-sdk/latest/)