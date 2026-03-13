/**
 * Nostrイベントペイロード
 * バックエンドから送信されるNostrイベントの型定義
 */
export interface NostrEventPayload {
  id: string;
  author: string;
  content: string;
  created_at: number;
  kind: number;
  tags: string[][];
}

/**
 * Nostrイベントの種類（kind）
 * https://github.com/nostr-protocol/nips/blob/master/01.md
 */
export enum NostrEventKind {
  // 基本的なイベント
  Metadata = 0, // ユーザーメタデータ
  TextNote = 1, // テキストノート
  RecommendRelay = 2, // リレー推奨
  ContactList = 3, // コンタクトリスト
  DirectMessage = 4, // ダイレクトメッセージ
  EventDeletion = 5, // イベント削除
  Repost = 6, // リポスト
  Reaction = 7, // リアクション

  // トピック関連（カスタム）
  TopicPost = 30078, // トピック投稿
  TopicMetadata = 30030, // トピックメタデータ
}

/**
 * リレー接続状態
 */
export interface RelayStatus {
  url: string;
  connected: boolean;
  latency?: number;
  last_check?: number;
}
