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

export interface TrendingTopic {
  topic_id: string;
  name: string;
  description: string | null;
  member_count: number;
  post_count: number;
  trending_score: number;
  rank: number;
  score_change: number | null;
}

export interface ListTrendingTopicsResult {
  generated_at: number;
  topics: TrendingTopic[];
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
  author_npub: string;
  topic_id: string;
  created_at: number;
  likes: number;
  boosts: number;
  replies: number;
  is_synced: boolean;
}

export interface ListTrendingPostsRequestParams {
  topicIds: string[];
  perTopic?: number;
}

export interface TrendingTopicPosts {
  topic_id: string;
  topic_name: string;
  relative_rank: number;
  posts: Post[];
}

export interface ListTrendingPostsResult {
  generated_at: number;
  topics: TrendingTopicPosts[];
}

export interface CreatePostRequest {
  content: string;
  topic_id?: string;
  tags?: string[][];
  reply_to?: string;
  quoted_post?: string;
}

export interface PaginationRequest {
  limit?: number;
  offset?: number;
}

export interface GetPostsRequest {
  topic_id?: string;
  author_pubkey?: string;
  pagination?: PaginationRequest;
  limit?: number;
  offset?: number;
}

export interface UserProfile {
  npub: string;
  pubkey: string;
  name?: string | null;
  display_name?: string | null;
  about?: string | null;
  picture?: string | null;
  banner?: string | null;
  website?: string | null;
  nip05?: string | null;
}

export interface UserListPage {
  items: UserProfile[];
  nextCursor: string | null;
  hasMore: boolean;
  totalCount: number;
}

export interface GetFollowersParams {
  npub: string;
  cursor?: string | null;
  limit?: number;
  sort?: FollowListSort;
  search?: string;
}

export interface GetFollowingParams {
  npub: string;
  cursor?: string | null;
  limit?: number;
  sort?: FollowListSort;
  search?: string;
}

export type FollowListSort = 'recent' | 'oldest' | 'name_asc' | 'name_desc';

export interface SendDirectMessagePayload {
  recipientNpub: string;
  content: string;
  clientMessageId?: string;
}

export interface DirectMessageItem {
  eventId: string | null;
  clientMessageId: string | null;
  senderNpub: string;
  recipientNpub: string;
  content: string;
  createdAt: number;
  delivered: boolean;
}

export interface DirectMessagePage {
  items: DirectMessageItem[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface DirectMessageConversationSummary {
  conversationNpub: string;
  unreadCount: number;
  lastReadAt: number;
  lastMessage: DirectMessageItem | null;
}

export interface DirectMessageConversationList {
  items: DirectMessageConversationSummary[];
}

export interface ListDirectMessageConversationsParams {
  limit?: number;
}

export interface ListDirectMessagesParams {
  conversationNpub: string;
  cursor?: string | null;
  limit?: number;
  direction?: 'backward' | 'forward';
}

export interface SendDirectMessageResult {
  eventId: string | null;
  queued: boolean;
}

export interface ListFollowingFeedParams {
  cursor?: string | null;
  limit?: number;
  includeReactions?: boolean;
}

export interface FollowingFeedPage {
  items: Post[];
  next_cursor: string | null;
  has_more: boolean;
  server_time: number;
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

  static async listTrendingTopics(limit?: number): Promise<ListTrendingTopicsResult> {
    const payload = limit !== undefined ? { request: { limit } } : undefined;
    return await invokeCommand<ListTrendingTopicsResult>('list_trending_topics', payload);
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
    const { topic_id, author_pubkey, pagination, limit, offset } = request;
    const payload: {
      request: {
        topic_id?: string;
        author_pubkey?: string;
        pagination?: PaginationRequest;
      };
    } = {
      request: {},
    };

    if (topic_id) {
      payload.request.topic_id = topic_id;
    }

    if (author_pubkey) {
      payload.request.author_pubkey = author_pubkey;
    }

    if (pagination) {
      payload.request.pagination = pagination;
    } else if (limit !== undefined || offset !== undefined) {
      payload.request.pagination = {
        limit,
        offset,
      };
    }

    return await invokeCommand<Post[]>('get_posts', payload);
  }

  static async listTrendingPosts(
    params: ListTrendingPostsRequestParams,
  ): Promise<ListTrendingPostsResult> {
    const request: Record<string, unknown> = {
      topic_ids: params.topicIds,
    };
    if (params.perTopic !== undefined) {
      request.per_topic = params.perTopic;
    }
    return await invokeCommand<ListTrendingPostsResult>('list_trending_posts', { request });
  }

  static async listFollowingFeed(params: ListFollowingFeedParams = {}): Promise<FollowingFeedPage> {
    const request: Record<string, unknown> = {
      cursor: params.cursor ?? null,
    };
    if (params.limit !== undefined) {
      request.limit = params.limit;
    }
    if (params.includeReactions !== undefined) {
      request.include_reactions = params.includeReactions;
    }
    return await invokeCommand<FollowingFeedPage>('list_following_feed', { request });
  }

  static async createPost(request: CreatePostRequest): Promise<Post> {
    return await invokeCommand<Post>('create_post', { request });
  }

  static async deletePost(id: string, reason?: string): Promise<void> {
    await invokeCommandVoid('delete_post', {
      request: {
        post_id: id,
        reason: reason ?? null,
      },
    });
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
    const bytes = options.data instanceof Uint8Array ? Array.from(options.data) : [...options.data];
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

  // ユーザー関連
  static async searchUsers(query: string, limit = 20): Promise<UserProfile[]> {
    return await invokeCommand<UserProfile[]>('search_users', { query, limit });
  }

  static async getUserProfile(npub: string): Promise<UserProfile | null> {
    return await invokeCommand<UserProfile | null>('get_user', { npub });
  }

  static async getUserProfileByPubkey(pubkey: string): Promise<UserProfile | null> {
    return await invokeCommand<UserProfile | null>('get_user_by_pubkey', { pubkey });
  }

  static async getFollowers(params: GetFollowersParams): Promise<UserListPage> {
    const response = await invokeCommand<{
      items: UserProfile[];
      next_cursor: string | null;
      has_more: boolean;
      total_count: number;
    }>('get_followers', {
      request: {
        npub: params.npub,
        cursor: params.cursor ?? null,
        limit: params.limit,
        sort: params.sort,
        search: params.search && params.search.trim().length > 0 ? params.search.trim() : null,
      },
    });

    return {
      items: response?.items ?? [],
      nextCursor: response?.next_cursor ?? null,
      hasMore: response?.has_more ?? false,
      totalCount: response?.total_count ?? 0,
    };
  }

  static async getFollowing(params: GetFollowingParams): Promise<UserListPage> {
    const response = await invokeCommand<{
      items: UserProfile[];
      next_cursor: string | null;
      has_more: boolean;
      total_count: number;
    }>('get_following', {
      request: {
        npub: params.npub,
        cursor: params.cursor ?? null,
        limit: params.limit,
        sort: params.sort,
        search: params.search && params.search.trim().length > 0 ? params.search.trim() : null,
      },
    });

    return {
      items: response?.items ?? [],
      nextCursor: response?.next_cursor ?? null,
      hasMore: response?.has_more ?? false,
      totalCount: response?.total_count ?? 0,
    };
  }

  static async sendDirectMessage(
    payload: SendDirectMessagePayload,
  ): Promise<SendDirectMessageResult> {
    const response = await invokeCommand<{
      event_id: string | null;
      queued: boolean;
    }>('send_direct_message', {
      request: {
        recipient_npub: payload.recipientNpub,
        content: payload.content,
        client_message_id: payload.clientMessageId,
      },
    });

    return {
      eventId: response?.event_id ?? null,
      queued: response?.queued ?? false,
    };
  }

  static async listDirectMessages(params: ListDirectMessagesParams): Promise<DirectMessagePage> {
    const response = await invokeCommand<{
      items: Array<{
        event_id: string | null;
        client_message_id: string | null;
        sender_npub: string;
        recipient_npub: string;
        content: string;
        created_at: number;
        delivered: boolean;
      }>;
      next_cursor: string | null;
      has_more: boolean;
    }>('list_direct_messages', {
      request: {
        conversation_npub: params.conversationNpub,
        cursor: params.cursor ?? null,
        limit: params.limit,
        direction: params.direction,
      },
    });

    return {
      items:
        response?.items?.map((item) => ({
          eventId: item.event_id,
          clientMessageId: item.client_message_id,
          senderNpub: item.sender_npub,
          recipientNpub: item.recipient_npub,
          content: item.content,
          createdAt: item.created_at,
          delivered: item.delivered,
        })) ?? [],
      nextCursor: response?.next_cursor ?? null,
      hasMore: response?.has_more ?? false,
    };
  }

  static async listDirectMessageConversations(
    params: ListDirectMessageConversationsParams = {},
  ): Promise<DirectMessageConversationList> {
    const response = await invokeCommand<{
      items: Array<{
        conversation_npub: string;
        unread_count: number;
        last_read_at: number;
        last_message: null | {
          event_id: string | null;
          client_message_id: string | null;
          sender_npub: string;
          recipient_npub: string;
          content: string;
          created_at: number;
          delivered: boolean;
        };
      }>;
    }>('list_direct_message_conversations', {
      request: {
        limit: params.limit,
      },
    });

    return {
      items:
        response?.items?.map((item) => ({
          conversationNpub: item.conversation_npub,
          unreadCount: item.unread_count,
          lastReadAt: item.last_read_at,
          lastMessage: item.last_message
            ? {
                eventId: item.last_message.event_id,
                clientMessageId: item.last_message.client_message_id,
                senderNpub: item.last_message.sender_npub,
                recipientNpub: item.last_message.recipient_npub,
                content: item.last_message.content,
                createdAt: item.last_message.created_at,
                delivered: item.last_message.delivered,
              }
            : null,
        })) ?? [],
    };
  }

  static async markDirectMessageConversationRead(params: {
    conversationNpub: string;
    lastReadAt: number;
  }): Promise<void> {
    await invokeCommandVoid('mark_direct_message_conversation_read', {
      request: {
        conversation_npub: params.conversationNpub,
        last_read_at: Math.max(0, Math.floor(params.lastReadAt)),
      },
    });
  }

  static async followUser(followerNpub: string, targetNpub: string): Promise<void> {
    await invokeCommandVoid('follow_user', {
      follower_npub: followerNpub,
      target_npub: targetNpub,
    });
  }

  static async unfollowUser(followerNpub: string, targetNpub: string): Promise<void> {
    await invokeCommandVoid('unfollow_user', {
      follower_npub: followerNpub,
      target_npub: targetNpub,
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
