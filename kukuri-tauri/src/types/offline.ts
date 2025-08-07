// オフライン関連の型定義

export interface OfflineAction {
  id: number;
  userPubkey: string;
  actionType: string;
  targetId?: string;
  actionData: string;
  localId: string;
  remoteId?: string;
  isSynced: boolean;
  createdAt: number;
  syncedAt?: number;
}

export interface SaveOfflineActionRequest {
  userPubkey: string;
  actionType: string;
  targetId?: string;
  actionData: Record<string, any>;
}

export interface SaveOfflineActionResponse {
  localId: string;
  action: OfflineAction;
}

export interface GetOfflineActionsRequest {
  userPubkey?: string;
  isSynced?: boolean;
  limit?: number;
}

export interface SyncOfflineActionsRequest {
  userPubkey: string;
}

export interface SyncOfflineActionsResponse {
  syncedCount: number;
  failedCount: number;
  pendingCount: number;
}

export interface CacheMetadata {
  id: number;
  cacheKey: string;
  cacheType: string;
  lastSyncedAt?: number;
  lastAccessedAt?: number;
  dataVersion: number;
  isStale: boolean;
  expiryTime?: number;
  metadata?: string;
}

export interface CacheStatusResponse {
  totalItems: number;
  staleItems: number;
  cacheTypes: CacheTypeStatus[];
}

export interface CacheTypeStatus {
  cacheType: string;
  itemCount: number;
  lastSyncedAt?: number;
  isStale: boolean;
}

export interface AddToSyncQueueRequest {
  actionType: string;
  payload: Record<string, any>;
}

export interface UpdateCacheMetadataRequest {
  cacheKey: string;
  cacheType: string;
  metadata?: Record<string, any>;
  expirySeconds?: number;
}

export interface OptimisticUpdate {
  id: number;
  updateId: string;
  entityType: string;
  entityId: string;
  originalData?: string;
  updatedData: string;
  isConfirmed: boolean;
  createdAt: number;
  confirmedAt?: number;
}

export interface SyncStatus {
  id: number;
  entityType: string;
  entityId: string;
  localVersion: number;
  remoteVersion?: number;
  lastLocalUpdate: number;
  lastRemoteSync?: number;
  syncStatus: 'synced' | 'pending' | 'conflict' | 'error';
  conflictData?: string;
}

// オフライン状態の型
export interface OfflineState {
  isOnline: boolean;
  lastSyncedAt?: number;
  pendingActions: OfflineAction[];
  syncQueue: AddToSyncQueueRequest[];
  optimisticUpdates: Map<string, OptimisticUpdate>;
  isSyncing: boolean;
  syncErrors: Map<string, string>;
}

// アクションタイプの列挙
export enum OfflineActionType {
  CREATE_POST = 'create_post',
  LIKE = 'like',
  BOOST = 'boost',
  BOOKMARK = 'bookmark',
  UNBOOKMARK = 'unbookmark',
  FOLLOW = 'follow',
  UNFOLLOW = 'unfollow',
  TOPIC_JOIN = 'topic_join',
  TOPIC_LEAVE = 'topic_leave',
  TOPIC_CREATE = 'topic_create',
  TOPIC_UPDATE = 'topic_update',
  TOPIC_DELETE = 'topic_delete',
  PROFILE_UPDATE = 'profile_update',
}

// エンティティタイプの列挙
export enum EntityType {
  POST = 'post',
  REACTION = 'reaction',
  TOPIC_MEMBERSHIP = 'topic_membership',
  USER = 'user',
  TOPIC = 'topic',
}