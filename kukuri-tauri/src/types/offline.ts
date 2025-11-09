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
  actionType: OfflineActionType;
  entityType: EntityType;
  entityId: string;
  data: string;
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
  total_items: number;
  stale_items: number;
  cache_types: CacheTypeStatus[];
}

export interface CacheTypeStatus {
  cache_type: string;
  item_count: number;
  last_synced_at?: number;
  is_stale: boolean;
  metadata?: Record<string, unknown> | null;
}

export interface AddToSyncQueueRequest {
  action_type: string;
  payload: Record<string, any>;
  priority?: number;
}

export interface UpdateCacheMetadataRequest {
  cacheKey: string;
  cacheType: string;
  metadata?: Record<string, any>;
  expirySeconds?: number;
  isStale?: boolean;
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

export interface SyncConflictDigest {
  entity_type: string;
  entity_id: string;
  sync_status: string;
}

export interface OfflineReindexReport {
  offline_action_count: number;
  queued_action_count: number;
  pending_queue_count: number;
  stale_cache_keys: string[];
  optimistic_update_ids: string[];
  sync_conflicts: SyncConflictDigest[];
  queued_offline_action_ids: string[];
  emitted_at: number;
}

// アクションタイプの列挙
export enum OfflineActionType {
  CREATE_POST = 'create_post',
  LIKE_POST = 'like_post',
  LIKE = 'like',
  BOOST = 'boost',
  BOOKMARK = 'bookmark',
  UNBOOKMARK = 'unbookmark',
  FOLLOW = 'follow',
  UNFOLLOW = 'unfollow',
  JOIN_TOPIC = 'join_topic',
  LEAVE_TOPIC = 'leave_topic',
  TOPIC_JOIN = 'topic_join',
  TOPIC_LEAVE = 'topic_leave',
  TOPIC_CREATE = 'topic_create',
  TOPIC_UPDATE = 'topic_update',
  TOPIC_DELETE = 'topic_delete',
  PROFILE_UPDATE = 'profile_update',
  DELETE_POST = 'delete_post',
}

// エンティティタイプの列挙
export enum EntityType {
  POST = 'post',
  REACTION = 'reaction',
  TOPIC_MEMBERSHIP = 'topic_membership',
  USER = 'user',
  TOPIC = 'topic',
}
