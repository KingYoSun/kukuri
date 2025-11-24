import type { ProfileAvatarAccessLevel, PendingTopic } from '@/lib/api/tauri';

export interface UserAvatarMetadata {
  blobHash: string;
  format: string;
  sizeBytes: number;
  accessLevel: ProfileAvatarAccessLevel;
  shareTicket: string;
  docVersion: number;
  updatedAt: string;
  contentSha256: string;
  nostrUri: string;
}

export interface User {
  id: string;
  pubkey: string;
  npub: string;
  name: string;
  displayName: string;
  picture: string;
  about: string;
  nip05: string;
  publicProfile: boolean;
  showOnlineStatus: boolean;
  avatar?: UserAvatarMetadata | null;
}

// Profile は User のエイリアス
export type Profile = User;

export interface Topic {
  id: string;
  name: string;
  description: string;
  tags: string[];
  memberCount: number;
  postCount: number;
  lastActive?: number;
  isActive: boolean;
  createdAt: Date;
  visibility?: 'public' | 'private';
  isJoined?: boolean;
}

export interface Post {
  id: string;
  content: string;
  author: User;
  topicId: string;
  created_at: number;
  tags: string[];
  likes: number;
  boosts: number;
  replies: Post[] | number;
  replyCount?: number;
  isSynced?: boolean; // P2Pネットワークに同期済みかどうか
  isBoosted?: boolean; // 現在のユーザーがブーストしたか
  isBookmarked?: boolean; // 現在のユーザーがブックマークしたか
  localId?: string; // ローカルで生成されたID（オフライン時の追跡用）
}

export interface AuthState {
  isAuthenticated: boolean;
  currentUser: User | null;
  privateKey: string | null;
}

export interface TopicState {
  topics: Map<string, Topic>;
  currentTopic: Topic | null;
  joinedTopics: string[];
  topicUnreadCounts: Map<string, number>;
  topicLastReadAt: Map<string, number>;
  pendingTopics: Map<string, PendingTopic>;
}

export interface PostState {
  posts: Map<string, Post>;
  postsByTopic: Map<string, string[]>;
}
