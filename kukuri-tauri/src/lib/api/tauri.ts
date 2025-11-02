import { invokeCommand, invokeCommandVoid } from './tauriClient';

// 認証関連の型定義
export interface GenerateKeypairResponse {
  public_key: string;
  nsec: string;
  npub: string;
}

export interface LoginRequest {
  nsec: string;
}

export interface LoginResponse {
  public_key: string;
  npub: string;
}

// トピック関連の型定義
export interface Topic {
  id: string;
  name: string;
  description: string;
  created_at: number;
  updated_at: number;
  member_count?: number;
  post_count?: number;
}

export interface CreateTopicRequest {
  name: string;
  description: string;
}

export interface TopicStats {
  topic_id: string;
  member_count: number;
  post_count: number;
  active_users_24h: number;
  trending_score: number;
}

export interface UpdateTopicRequest {
  id: string;
  name: string;
  description: string;
}

// ポスト関連の型定義
export interface Post {
  id: string;
  content: string;
  author_pubkey: string;
  topic_id: string;
  created_at: number;
  likes: number;
  boosts: number;
  replies: number;
  is_synced: boolean;
}

export interface CreatePostRequest {
  content: string;
  topic_id?: string;
  tags?: string[][];
  reply_to?: string;
  quoted_post?: string;
}

export interface GetPostsRequest {
  topic_id?: string;
  limit?: number;
  offset?: number;
}

export type ProfileAvatarAccessLevel = 'public' | 'contacts_only' | 'private';

export interface UploadProfileAvatarOptions {
  npub: string;
  data: Uint8Array | number[];
  format: string;
  accessLevel: ProfileAvatarAccessLevel;
}

export interface UploadProfileAvatarResult {
  npub: string;
  blob_hash: string;
  format: string;
  size_bytes: number;
  access_level: ProfileAvatarAccessLevel;
  share_ticket: string;
  doc_version: number;
  updated_at: string;
  content_sha256: string;
}

export interface FetchProfileAvatarResult {
  npub: string;
  blob_hash: string;
  format: string;
  size_bytes: number;
  access_level: ProfileAvatarAccessLevel;
  share_ticket: string;
  doc_version: number;
  updated_at: string;
  content_sha256: string;
  data_base64: string;
}

// Tauri API クラス
export class TauriApi {
  // 認証関連
  static async generateKeypair(): Promise<GenerateKeypairResponse> {
    return await invokeCommand<GenerateKeypairResponse>('generate_keypair');
  }

  static async login(request: LoginRequest): Promise<LoginResponse> {
    return await invokeCommand<LoginResponse>('login', { request });
  }

  static async logout(): Promise<void> {
    await invokeCommandVoid('logout');
  }

  // トピック関連
  static async getTopics(): Promise<Topic[]> {
    return await invokeCommand<Topic[]>('get_topics');
  }

  static async getTopicStats(topicId: string): Promise<TopicStats> {
    return await invokeCommand<TopicStats>('get_topic_stats', {
      request: { topic_id: topicId },
    });
  }

  static async createTopic(request: CreateTopicRequest): Promise<Topic> {
    return await invokeCommand<Topic>('create_topic', { request });
  }

  static async updateTopic(request: UpdateTopicRequest): Promise<Topic> {
    return await invokeCommand<Topic>('update_topic', { request });
  }

  static async deleteTopic(id: string): Promise<void> {
    await invokeCommandVoid('delete_topic', { request: { id } });
  }

  static async joinTopic(topicId: string): Promise<void> {
    await invokeCommandVoid('join_topic', { request: { topic_id: topicId } });
  }

  static async leaveTopic(topicId: string): Promise<void> {
    await invokeCommandVoid('leave_topic', { request: { topic_id: topicId } });
  }

  // ポスト関連
  static async getPosts(request: GetPostsRequest = {}): Promise<Post[]> {
    return await invokeCommand<Post[]>('get_posts', { request });
  }

  static async createPost(request: CreatePostRequest): Promise<Post> {
    return await invokeCommand<Post>('create_post', { request });
  }

  static async deletePost(id: string): Promise<void> {
    await invokeCommandVoid('delete_post', { id });
  }

  static async likePost(postId: string): Promise<void> {
    await invokeCommandVoid('like_post', { postId });
  }

  static async boostPost(postId: string): Promise<void> {
    await invokeCommandVoid('boost_post', { postId });
  }

  static async bookmarkPost(postId: string): Promise<void> {
    await invokeCommandVoid('bookmark_post', { postId });
  }

  static async unbookmarkPost(postId: string): Promise<void> {
    await invokeCommandVoid('unbookmark_post', { postId });
  }

  static async getBookmarkedPostIds(): Promise<string[]> {
    return await invokeCommand<string[]>('get_bookmarked_post_ids');
  }

  static async uploadProfileAvatar(
    options: UploadProfileAvatarOptions,
  ): Promise<UploadProfileAvatarResult> {
    const bytes =
      options.data instanceof Uint8Array ? Array.from(options.data) : [...options.data];
    return await invokeCommand<UploadProfileAvatarResult>('upload_profile_avatar', {
      request: {
        npub: options.npub,
        bytes,
        format: options.format,
        access_level: options.accessLevel,
      },
    });
  }

  static async fetchProfileAvatar(npub: string): Promise<FetchProfileAvatarResult> {
    return await invokeCommand<FetchProfileAvatarResult>('fetch_profile_avatar', {
      request: { npub },
    });
  }
}

// Nostr関連の型定義
export interface NostrMetadata {
  name?: string;
  display_name?: string;
  about?: string;
  picture?: string;
  banner?: string;
  nip05?: string;
  lud16?: string;
  website?: string;
}

export interface RelayInfo {
  url: string;
  status: string;
}

export interface NostrEvent {
  id: string;
  author: string;
  content: string;
  created_at: number;
  kind: number;
  tags: string[][];
}

interface EventCommandResponse {
  event_id: string;
  success: boolean;
  message?: string | null;
}

// Nostr API
export class NostrAPI {
  static async initialize(): Promise<void> {
    await invokeCommandVoid('initialize_nostr');
  }

  static async addRelay(url: string): Promise<void> {
    await invokeCommandVoid('add_relay', { url });
  }

  static async publishTextNote(content: string): Promise<string> {
    const response = await invokeCommand<EventCommandResponse>('publish_text_note', { content });
    return response.event_id;
  }

  static async publishTopicPost(
    topicId: string,
    content: string,
    replyTo?: string,
  ): Promise<string> {
    const response = await invokeCommand<EventCommandResponse>('publish_topic_post', {
      topicId,
      content,
      replyTo,
    });
    return response.event_id;
  }

  static async sendReaction(eventId: string, reaction: string): Promise<string> {
    const response = await invokeCommand<EventCommandResponse>('send_reaction', {
      eventId,
      reaction,
    });
    return response.event_id;
  }

  static async updateMetadata(metadata: NostrMetadata): Promise<string> {
    const response = await invokeCommand<EventCommandResponse>('update_nostr_metadata', {
      metadata,
    });
    return response.event_id;
  }

  static async subscribeToTopic(topicId: string): Promise<void> {
    await invokeCommandVoid('subscribe_to_topic', { topicId });
  }

  static async subscribeToUser(pubkey: string): Promise<void> {
    await invokeCommandVoid('subscribe_to_user', { pubkey });
  }

  static async getNostrPubkey(): Promise<string | null> {
    const response = await invokeCommand<{ pubkey: string | null }>('get_nostr_pubkey');
    return response.pubkey ?? null;
  }

  static async listSubscriptions(): Promise<{
    subscriptions: Array<{
      target: string;
      target_type: 'topic' | 'user';
      status: string;
      last_synced_at: number | null;
      last_attempt_at: number | null;
      failure_count: number;
      error_message: string | null;
    }>;
  }> {
    return await invokeCommand('list_nostr_subscriptions');
  }

  static async deleteEvents(eventIds: string[], reason?: string): Promise<string> {
    const response = await invokeCommand<EventCommandResponse>('delete_events', {
      eventIds,
      reason,
    });
    return response.event_id;
  }

  static async disconnect(): Promise<void> {
    await invokeCommandVoid('disconnect_nostr');
  }

  static async getRelayStatus(): Promise<RelayInfo[]> {
    return await invokeCommand<RelayInfo[]>('get_relay_status');
  }
}
