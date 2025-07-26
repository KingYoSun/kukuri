import { invoke } from '@tauri-apps/api/core';

// 認証関連の型定義
export interface GenerateKeypairResponse {
  public_key: string;
  nsec: string;
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
}

export interface CreateTopicRequest {
  name: string;
  description: string;
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
  replies: number;
}

export interface CreatePostRequest {
  content: string;
  topic_id: string;
}

export interface GetPostsRequest {
  topic_id?: string;
  limit?: number;
  offset?: number;
}

// Tauri API クラス
export class TauriApi {
  // 認証関連
  static async generateKeypair(): Promise<GenerateKeypairResponse> {
    return await invoke('generate_keypair');
  }

  static async login(request: LoginRequest): Promise<LoginResponse> {
    return await invoke('login', { request });
  }

  static async logout(): Promise<void> {
    return await invoke('logout');
  }

  // トピック関連
  static async getTopics(): Promise<Topic[]> {
    return await invoke('get_topics');
  }

  static async createTopic(request: CreateTopicRequest): Promise<Topic> {
    return await invoke('create_topic', { request });
  }

  static async updateTopic(request: UpdateTopicRequest): Promise<Topic> {
    return await invoke('update_topic', { request });
  }

  static async deleteTopic(id: string): Promise<void> {
    return await invoke('delete_topic', { id });
  }

  // ポスト関連
  static async getPosts(request: GetPostsRequest = {}): Promise<Post[]> {
    return await invoke('get_posts', { request });
  }

  static async createPost(request: CreatePostRequest): Promise<Post> {
    return await invoke('create_post', { request });
  }

  static async deletePost(id: string): Promise<void> {
    return await invoke('delete_post', { id });
  }

  static async likePost(postId: string): Promise<void> {
    return await invoke('like_post', { postId });
  }
}