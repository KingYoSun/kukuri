import { SecureStorageApi } from '@/lib/api/secureStorage';
import {
  TauriApi,
  NostrAPI,
  type SeedDirectMessageConversationResult,
  type PendingTopic,
  type UserProfile,
} from '@/lib/api/tauri';
import { TauriCommandError } from '@/lib/api/tauriClient';
import { p2pApi } from '@/lib/api/p2p';
import { accessControlApi } from '@/lib/api/accessControl';
import {
  communityNodeApi,
  defaultCommunityNodeRoles,
  type GroupKeyEntry,
} from '@/lib/api/communityNode';
import { errorHandler } from '@/lib/errorHandler';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import {
  followingFeedQueryKey,
  trendingPostsQueryKey,
  trendingTopicsQueryKey,
  type FollowingFeedPageResult,
  type TrendingPostsResult,
  type TrendingTopicsResult,
} from '@/hooks/useTrendingFeeds';
import { queryClient } from '@/lib/queryClient';
import { persistKeys } from '@/stores/config/persist';
import {
  clearFallbackAccounts,
  listFallbackAccountMetadata,
  useAuthStore,
} from '@/stores/authStore';
import { usePostStore, type Post } from '@/stores';
import { mapApiMessageToModel, useDirectMessageStore } from '@/stores/directMessageStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { useTopicStore } from '@/stores/topicStore';
import { getE2EStatus, setE2EStatus, type E2EStatus } from './e2eStatus';
import { offlineApi } from '@/api/offline';
import { EntityType, OfflineActionType } from '@/types/offline';
import { v4 as uuidv4 } from 'uuid';

type AuthSnapshot = {
  currentUser: ReturnType<typeof useAuthStore.getState>['currentUser'];
  accounts: ReturnType<typeof useAuthStore.getState>['accounts'];
  isAuthenticated: boolean;
  hasPrivateKey: boolean;
  fallbackAccounts: ReturnType<typeof listFallbackAccountMetadata>;
};

interface OfflineSnapshot {
  isOnline: boolean;
  isSyncing: boolean;
  lastSyncedAt: number | null;
  pendingActionCount: number;
}

interface DirectMessageSnapshot {
  unreadCounts: Record<string, number>;
  unreadTotal: number;
  conversations: Record<string, number>;
  conversationKeys: string[];
  latestConversationNpub: string | null;
  activeConversationNpub: string | null;
  isInboxOpen: boolean;
  isDialogOpen: boolean;
}

interface TrendingFixturePost {
  id?: string;
  title: string;
  author?: string;
}

interface TrendingFixtureTopic {
  topicId?: string;
  title: string;
  description?: string;
  posts?: TrendingFixturePost[];
}

interface TrendingFixturePayload {
  topics: TrendingFixtureTopic[];
}

interface SeedTrendingFixtureResult {
  topics: Array<{
    id: string;
    name: string;
    author: string;
  }>;
  authors: Array<{
    name: string;
    npub: string;
  }>;
  followerNpub: string;
}

interface UserSearchFixtureUser {
  displayName: string;
  about?: string;
  follow?: boolean;
}

interface SeedUserSearchFixtureResult {
  users: Array<{
    npub: string;
    displayName: string;
    about: string;
    isFollowed: boolean;
  }>;
}

interface PrimeUserSearchRateLimitResult {
  attempts: number;
  retryAfterSeconds: number | null;
  triggered: boolean;
}

interface SeedCommunityNodePostPayload {
  id: string;
  content: string;
  authorPubkey: string;
  authorNpub?: string;
  authorName?: string;
  authorDisplayName?: string;
  topicId: string;
  createdAt?: number;
}

interface SeedCommunityNodePostResult {
  id: string;
}

interface BootstrapSnapshot {
  source: string;
  effectiveNodes: string[];
  cliNodes: string[];
  cliUpdatedAtMs: number | null;
  envLocked: boolean;
}

type CommunityNodeAuth = Awaited<ReturnType<typeof communityNodeApi.authenticate>>;
type CommunityNodeConfig = Awaited<ReturnType<typeof communityNodeApi.getConfig>>;
type CommunityNodeConsents = Awaited<ReturnType<typeof communityNodeApi.getConsentStatus>>;

interface CommunityNodeAuthFlowResult {
  config: CommunityNodeConfig;
  auth: CommunityNodeAuth;
  consents: CommunityNodeConsents;
}

interface FriendPlusActor {
  npub: string;
  pubkey: string;
}

interface SeedFriendPlusAccountsResult {
  requester: FriendPlusActor;
  inviter: FriendPlusActor;
  friend: FriendPlusActor;
}

interface AccessControlRequestJoinPayload {
  topic_id?: string;
  scope?: string;
  invite_event_json?: unknown;
  target_pubkey?: string;
  broadcast_to_topic?: boolean;
}

interface AccessControlIngestEventPayload {
  event_json: unknown;
}

export interface E2EBridge {
  resetAppState: () => Promise<void>;
  getAuthSnapshot: () => AuthSnapshot;
  getOfflineSnapshot: () => OfflineSnapshot;
  setOnlineStatus: (isOnline: boolean) => { isOnline: boolean; pendingActionCount: number };
  seedOfflineActions: (payload: {
    topicId: string;
    includeConflict?: boolean;
    markOffline?: boolean;
  }) => { pendingActionCount: number; localIds: string[]; conflictedLocalId: string | null };
  enqueueSyncQueueItem: (payload?: {
    cacheType?: string;
    source?: string;
  }) => Promise<{ queueId: number; cacheType: string; requestedAt: string }>;
  ensureTestTopic: (payload?: {
    name?: string;
    topicId?: string;
  }) => Promise<{ id: string; name: string }>;
  seedCommunityNodePost: (
    payload: SeedCommunityNodePostPayload,
  ) => Promise<SeedCommunityNodePostResult>;
  clearOfflineState: () => Promise<{ pendingActionCount: number }>;
  getDirectMessageSnapshot: () => DirectMessageSnapshot;
  switchAccount: (npub: string) => Promise<void>;
  seedDirectMessageConversation: (payload?: {
    content?: string;
    createdAt?: number;
  }) => Promise<SeedDirectMessageConversationResult>;
  getTopicSnapshot: () => {
    topics: Array<{
      id: string;
      name: string;
      description?: string | null;
      postCount: number;
      memberCount: number;
      isJoined: boolean;
    }>;
    pendingTopics: Array<{
      pending_id: string;
      name: string;
      description?: string | null;
      status: string;
      offline_action_id: string;
      synced_topic_id?: string | null;
    }>;
    joinedTopics: string[];
    currentTopicId: string | null;
  };
  syncPendingTopicQueue: () => Promise<{
    pendingCountBefore: number;
    pendingCountAfter: number;
    createdTopicIds: string[];
  }>;
  seedTrendingFixture: (payload: TrendingFixturePayload) => Promise<SeedTrendingFixtureResult>;
  seedUserSearchFixture: (payload: {
    users: UserSearchFixtureUser[];
  }) => Promise<SeedUserSearchFixtureResult>;
  primeUserSearchRateLimit: (payload?: {
    query?: string;
    limit?: number;
  }) => Promise<PrimeUserSearchRateLimitResult>;
  getBootstrapSnapshot: () => Promise<BootstrapSnapshot>;
  applyCliBootstrap: () => Promise<BootstrapSnapshot>;
  clearBootstrapNodes: () => Promise<BootstrapSnapshot>;
  seedFriendPlusAccounts: () => Promise<SeedFriendPlusAccountsResult>;
  accessControlRequestJoin: (
    payload: AccessControlRequestJoinPayload,
  ) => Promise<Awaited<ReturnType<typeof accessControlApi.requestJoin>>>;
  accessControlListJoinRequests: () => Promise<
    Awaited<ReturnType<typeof accessControlApi.listJoinRequests>>
  >;
  accessControlApproveJoinRequest: (payload: {
    event_id: string;
  }) => Promise<Awaited<ReturnType<typeof accessControlApi.approveJoinRequest>>>;
  accessControlIngestEventJson: (payload: AccessControlIngestEventPayload) => Promise<void>;
  communityNodeAuthFlow: (payload: { baseUrl: string }) => Promise<CommunityNodeAuthFlowResult>;
  communityNodeListGroupKeys: () => Promise<GroupKeyEntry[]>;
  communityNodeListBootstrapNodes: () => Promise<Record<string, unknown>>;
  communityNodeListBootstrapServices: (payload: {
    topicId: string;
  }) => Promise<Record<string, unknown>>;
}

declare global {
  interface Window {
    __KUKURI_E2E__?: E2EBridge;
    __KUKURI_E2E_BOOTSTRAP__?: () => Promise<void> | void;
    __KUKURI_E2E_MESSAGE_HANDLER__?: boolean;
    __KUKURI_E2E_STATUS__?: E2EStatus;
  }
  interface Document {
    __KUKURI_E2E_DOM_BRIDGE__?: boolean;
  }
}

const PERSISTED_KEYS: string[] = [
  persistKeys.auth,
  persistKeys.drafts,
  persistKeys.offline,
  persistKeys.p2p,
  persistKeys.topic,
  persistKeys.ui,
  persistKeys.privacy,
  persistKeys.keyManagement,
  'kukuri-theme',
  'theme',
  'kukuri-locale',
  'kukuri-language',
  'i18nextLng',
];

const CHANNEL_ID = 'kukuri-e2e-channel';
const REQUEST_ATTR = 'data-e2e-request';
const RESPONSE_ATTR = 'data-e2e-response';
const READY_ATTR = 'data-e2e-ready';

type BridgeRequest = { requestId: string; action: keyof E2EBridge; args?: unknown };
type BridgeResponse = { requestId: string; result?: unknown; error?: string };

async function purgeSecureAccounts() {
  try {
    const accounts = await SecureStorageApi.listAccounts();
    for (const account of accounts) {
      try {
        await SecureStorageApi.removeAccount(account.npub);
      } catch (error) {
        errorHandler.log('E2EBridge.removeAccountFailed', error, {
          context: 'registerE2EBridge.purgeSecureAccounts',
          metadata: { npub: account.npub },
        });
      }
    }
  } catch (error) {
    errorHandler.log('E2EBridge.listAccountsFailed', error, {
      context: 'registerE2EBridge.purgeSecureAccounts',
    });
  }
}

function clearPersistedState() {
  if (typeof window === 'undefined') {
    return;
  }
  for (const key of PERSISTED_KEYS) {
    window.localStorage?.removeItem(key);
  }
}

async function resetAuthStore() {
  try {
    await useAuthStore.getState().logout();
  } catch (error) {
    errorHandler.log('E2EBridge.logoutFailed', error, {
      context: 'registerE2EBridge.resetAuthStore',
    });
  }
  clearFallbackAccounts();
  useAuthStore.setState({
    isAuthenticated: false,
    currentUser: null,
    privateKey: null,
    accounts: [],
  });
}

const bootstrapSnapshotFromConfig = (
  config: Awaited<ReturnType<typeof p2pApi.getBootstrapConfig>>,
): BootstrapSnapshot => ({
  source: config.source ?? 'none',
  effectiveNodes: config.effective_nodes ?? [],
  cliNodes: config.cli_nodes ?? [],
  cliUpdatedAtMs: config.cli_updated_at_ms ?? null,
  envLocked: Boolean(config.env_locked),
});

const refreshRelayStatusSafe = async () => {
  try {
    await useAuthStore.getState().updateRelayStatus();
  } catch (error) {
    errorHandler.log('E2EBridge.refreshRelayStatusFailed', error, {
      context: 'registerE2EBridge.refreshRelayStatusSafe',
    });
  }
};

export function registerE2EBridge(): void {
  if (typeof window === 'undefined') {
    return;
  }

  if (!window.__KUKURI_E2E_BOOTSTRAP__) {
    window.__KUKURI_E2E_BOOTSTRAP__ = registerE2EBridge;
  }

  setE2EStatus(getE2EStatus() ?? 'pending');

  try {
    if (!window.__KUKURI_E2E__) {
      window.__KUKURI_E2E__ = {
        resetAppState: async () => {
          await purgeSecureAccounts();
          clearPersistedState();
          await resetAuthStore();
        },
        getAuthSnapshot: () => {
          const state = useAuthStore.getState();
          return {
            currentUser: state.currentUser,
            accounts: state.accounts,
            isAuthenticated: state.isAuthenticated,
            hasPrivateKey: Boolean(state.privateKey),
            fallbackAccounts: listFallbackAccountMetadata(),
          };
        },
        switchAccount: async (npub: string) => {
          await useAuthStore.getState().switchAccount(npub);
          const switchedState = useAuthStore.getState();
          if (switchedState.currentUser?.npub !== npub) {
            throw new Error(`Failed to switch account to ${npub}`);
          }

          // Fallback 経路の switchAccount はフロント状態のみ切り替えるため、
          // privateKey が残っている場合は Rust 側のアクティブ鍵も同期する。
          const nsec = switchedState.privateKey;
          if (typeof nsec === 'string' && nsec.trim().length > 0) {
            await TauriApi.login({ nsec });
          }
        },
        getOfflineSnapshot: () => {
          const offlineState = useOfflineStore.getState();
          return {
            isOnline: offlineState.isOnline,
            isSyncing: offlineState.isSyncing,
            lastSyncedAt: offlineState.lastSyncedAt ?? null,
            pendingActionCount: offlineState.pendingActions.length,
          };
        },
        setOnlineStatus: (isOnline: boolean) => {
          const offlineStore = useOfflineStore.getState();
          offlineStore.setOnlineStatus(isOnline);
          const nextState = useOfflineStore.getState();
          return {
            isOnline: nextState.isOnline,
            pendingActionCount: nextState.pendingActions.length,
          };
        },
        seedOfflineActions: (payload) => {
          const authState = useAuthStore.getState();
          const topicId = payload?.topicId;
          if (!topicId) {
            throw new Error('topicId is required to seed offline actions');
          }
          if (!authState.currentUser?.npub) {
            throw new Error('Active account is required to seed offline actions');
          }

          const offlineStore = useOfflineStore.getState();
          if (payload?.markOffline !== false) {
            offlineStore.setOnlineStatus(false);
          }

          const now = Date.now();
          const actions: Array<{
            type: OfflineActionType;
            createdAt: number;
            data: Record<string, unknown>;
          }> = [
            {
              type: OfflineActionType.JOIN_TOPIC,
              createdAt: payload?.includeConflict ? now - 36 * 60 * 60 * 1000 : now,
              data: {
                topicId,
                entityType: EntityType.TOPIC,
                entityId: topicId,
              },
            },
            {
              type: OfflineActionType.CREATE_POST,
              createdAt: now,
              data: {
                topicId,
                entityType: EntityType.POST,
                entityId: `e2e-post-${now}`,
                content: 'E2E offline sync post',
                replyTo: null,
                quotedPost: null,
              },
            },
          ];

          const localIds: string[] = [];
          actions.forEach((action, index) => {
            const localId = `e2e-offline-${action.type}-${now + index}`;
            offlineStore.addPendingAction({
              id: now + index,
              userPubkey: authState.currentUser!.npub,
              actionType: action.type,
              targetId: topicId,
              actionData: JSON.stringify(action.data),
              localId,
              isSynced: false,
              createdAt: action.createdAt,
              syncedAt: undefined,
            });
            localIds.push(localId);
          });

          return {
            pendingActionCount: useOfflineStore.getState().pendingActions.length,
            localIds,
            conflictedLocalId: payload?.includeConflict ? (localIds[0] ?? null) : null,
          };
        },
        enqueueSyncQueueItem: async (payload) => {
          const cacheType = payload?.cacheType ?? 'sync_queue';
          const requestedAt = new Date().toISOString();
          const userPubkey = useAuthStore.getState().currentUser?.npub ?? 'unknown';
          const queueId = await offlineApi.addToSyncQueue({
            action_type: 'manual_sync_refresh',
            payload: {
              cacheType,
              requestedAt,
              source: payload?.source ?? 'e2e-bridge',
              requestedBy: userPubkey,
            },
            priority: 5,
          });
          try {
            await offlineApi.updateCacheMetadata({
              cacheKey: 'offline_actions',
              cacheType,
              metadata: {
                cacheType,
                requestedAt,
                requestedBy: userPubkey,
                queueItemId: queueId,
                source: payload?.source ?? 'e2e-bridge',
              },
              expirySeconds: 3600,
              isStale: true,
            });
          } catch (error) {
            errorHandler.log('E2EBridge.enqueueSyncQueue.metadataFailed', error, {
              context: 'registerE2EBridge.enqueueSyncQueueItem',
              metadata: { queueId, cacheType },
            });
          }
          return { queueId, cacheType, requestedAt };
        },
        ensureTestTopic: async (payload) => {
          const authState = useAuthStore.getState();
          if (!authState.currentUser?.npub) {
            throw new Error('Active account is required to create topics');
          }
          const desiredName =
            (payload?.name || '').trim() || `e2e-offline-topic-${Date.now().toString(36)}`;
          const explicitId = (payload?.topicId ?? '').trim();

          if (explicitId) {
            const existingById = Array.from(useTopicStore.getState().topics.values()).find(
              (topic) => topic.id === explicitId,
            );
            if (existingById) {
              return { id: existingById.id, name: existingById.name };
            }

            const now = new Date();
            useTopicStore.getState().addTopic({
              id: explicitId,
              name: desiredName,
              description: 'E2E invite topic',
              tags: [],
              memberCount: 1,
              postCount: 0,
              isActive: true,
              createdAt: now,
              visibility: 'public',
              isJoined: true,
            });
            return { id: explicitId, name: desiredName };
          }

          const existing = Array.from(useTopicStore.getState().topics.values()).find(
            (topic) => topic.name === desiredName,
          );
          if (existing) {
            return { id: existing.id, name: existing.name };
          }
          try {
            const created = await TauriApi.createTopic({
              name: desiredName,
              description: 'E2E offline sync topic',
              visibility: 'public',
            });
            try {
              await useTopicStore.getState().fetchTopics();
            } catch (error) {
              errorHandler.log('E2EBridge.ensureTestTopic.fetchFailed', error, {
                context: 'registerE2EBridge.ensureTestTopic',
              });
            }
            return { id: created.id, name: created.name ?? desiredName };
          } catch (error) {
            errorHandler.log('E2EBridge.ensureTestTopic.createFailed', error, {
              context: 'registerE2EBridge.ensureTestTopic',
              metadata: { name: desiredName },
            });
            const fallback = Array.from(useTopicStore.getState().topics.values())[0] ?? null;
            if (fallback) {
              return { id: fallback.id, name: fallback.name };
            }
            throw error;
          }
        },
        seedCommunityNodePost: async (payload) => {
          if (!payload?.id) {
            throw new Error('post id is required to seed community node post');
          }
          if (!payload?.authorPubkey) {
            throw new Error('authorPubkey is required to seed community node post');
          }
          if (!payload?.topicId) {
            throw new Error('topicId is required to seed community node post');
          }

          const createdAt =
            typeof payload.createdAt === 'number' && Number.isFinite(payload.createdAt)
              ? payload.createdAt
              : Math.floor(Date.now() / 1000);
          const authorName = (payload.authorName ?? '').trim();
          const authorDisplayName = (payload.authorDisplayName ?? '').trim();
          const author = applyKnownUserMetadata({
            id: payload.authorPubkey,
            pubkey: payload.authorPubkey,
            npub: payload.authorNpub ?? payload.authorPubkey,
            name: authorName || payload.authorPubkey,
            displayName: authorDisplayName || authorName || payload.authorPubkey,
            picture: '',
            about: '',
            nip05: '',
            publicProfile: false,
            showOnlineStatus: false,
          });
          const post: Post = {
            id: payload.id,
            content: payload.content ?? '',
            author,
            topicId: payload.topicId,
            scope: 'public',
            epoch: null,
            isEncrypted: false,
            created_at: createdAt,
            tags: [],
            likes: 0,
            boosts: 0,
            replies: [],
            replyCount: 0,
            isSynced: true,
          };

          usePostStore.getState().addPost(post);

          const upsertPostIntoList = (posts?: Post[]) => {
            const filtered = (posts ?? []).filter((item) => item.id !== post.id);
            return [...filtered, post].sort((a, b) => b.created_at - a.created_at);
          };

          queryClient.setQueryData<Post[]>(['timeline'], (prev) => upsertPostIntoList(prev));
          queryClient.setQueryData<Post[]>(['posts', post.topicId], (prev) =>
            upsertPostIntoList(prev),
          );

          return { id: post.id };
        },
        clearOfflineState: async () => {
          const offlineStore = useOfflineStore.getState();
          offlineStore.clearPendingActions();
          if (typeof useOfflineStore.setState === 'function') {
            useOfflineStore.setState((state) => ({
              ...state,
              syncErrors: new Map(),
            }));
          }
          offlineStore.updateLastSyncedAt();
          try {
            await offlineApi.updateCacheMetadata({
              cacheKey: 'offline_actions',
              cacheType: 'sync_queue',
              metadata: {
                cacheType: 'sync_queue',
                requestedAt: null,
                requestedBy: useAuthStore.getState().currentUser?.npub ?? 'unknown',
                queueItemId: null,
                source: 'e2e-bridge',
              },
              expirySeconds: 3600,
              isStale: false,
            });
          } catch (error) {
            errorHandler.log('E2EBridge.clearOfflineState.metadataFailed', error, {
              context: 'registerE2EBridge.clearOfflineState',
            });
          }
          return {
            pendingActionCount: useOfflineStore.getState().pendingActions.length,
          };
        },
        getDirectMessageSnapshot: () => {
          const state = useDirectMessageStore.getState();
          const unreadCounts = { ...state.unreadCounts };
          const conversations: Record<string, number> = {};
          for (const [npub, messages] of Object.entries(state.conversations)) {
            conversations[npub] = messages?.length ?? 0;
          }
          const latest = Object.entries(state.conversations)
            .map(([npub, messages]) => ({
              npub,
              last: messages && messages.length > 0 ? messages[messages.length - 1] : null,
            }))
            .filter((item) => item.last !== null)
            .sort((a, b) => (b.last?.createdAt ?? 0) - (a.last?.createdAt ?? 0))[0]?.npub;

          return {
            unreadCounts,
            unreadTotal: Object.values(unreadCounts).reduce((sum, value) => sum + value, 0),
            conversations,
            conversationKeys: Object.keys({
              ...state.conversations,
              ...state.unreadCounts,
            }),
            latestConversationNpub: latest ?? null,
            activeConversationNpub: state.activeConversationNpub,
            isInboxOpen: state.isInboxOpen,
            isDialogOpen: state.isDialogOpen,
          };
        },
        seedDirectMessageConversation: async (payload) => {
          const authState = useAuthStore.getState();
          const current = authState.currentUser;
          if (!current?.npub) {
            throw new Error('No active account for direct message seeding');
          }
          let recipientNsec: string | undefined =
            typeof authState.privateKey === 'string' && authState.privateKey.trim().length > 0
              ? authState.privateKey
              : undefined;
          try {
            recipientNsec = await TauriApi.exportPrivateKey(current.npub);
          } catch (error) {
            errorHandler.log('E2EBridge.dmSeedExportFailed', error, {
              context: 'registerE2EBridge.seedDirectMessageConversation',
              metadata: { npub: current.npub },
            });
          }
          const result = await TauriApi.seedDirectMessageConversation({
            ...(payload ?? {}),
            recipientNsec,
          });
          const fallbackSummary = {
            conversationNpub: result.conversationNpub,
            unreadCount: 1,
            lastReadAt: 0,
            lastMessage: {
              eventId: null,
              clientMessageId: `seed-${result.conversationNpub}-${result.createdAt}`,
              senderNpub: result.conversationNpub,
              recipientNpub: current.npub,
              content: result.content,
              createdAt: result.createdAt,
              status: 'sent' as const,
            },
          };
          try {
            useDirectMessageStore
              .getState()
              .receiveIncomingMessage(result.conversationNpub, fallbackSummary.lastMessage, {
                incrementUnread: true,
              });
            useDirectMessageStore
              .getState()
              .setMessages(result.conversationNpub, [fallbackSummary.lastMessage], {
                replace: false,
              });
          } catch (error) {
            errorHandler.log('E2EBridge.dmSeedReceiveFailed', error, {
              context: 'registerE2EBridge.seedDirectMessageConversation',
            });
          }
          try {
            const conversations = await TauriApi.listDirectMessageConversations({
              cursor: null,
              limit: 50,
            });
            let summaries = conversations.items.map((item) => ({
              conversationNpub: item.conversationNpub,
              unreadCount: item.unreadCount,
              lastReadAt: item.lastReadAt,
              lastMessage: item.lastMessage ? mapApiMessageToModel(item.lastMessage) : undefined,
            }));
            if (!summaries.some((item) => item.conversationNpub === result.conversationNpub)) {
              summaries = [fallbackSummary, ...summaries];
            }
            useDirectMessageStore.getState().hydrateConversations(summaries);
          } catch (error) {
            errorHandler.log('E2EBridge.dmSeedHydrationFailed', error, {
              context: 'registerE2EBridge.seedDirectMessageConversation',
            });
            useDirectMessageStore.getState().hydrateConversations([fallbackSummary]);
          }
          return result;
        },
        getTopicSnapshot: () => {
          const topicState = useTopicStore.getState();
          return {
            topics: Array.from(topicState.topics.values()).map((topic) => ({
              id: topic.id,
              name: topic.name,
              description: topic.description ?? null,
              postCount: topic.postCount ?? 0,
              memberCount: topic.memberCount ?? 0,
              isJoined: Boolean(topic.isJoined),
            })),
            pendingTopics: Array.from(topicState.pendingTopics.values()).map((pending) => ({
              pending_id: pending.pending_id,
              name: pending.name,
              description: pending.description ?? null,
              status: pending.status,
              offline_action_id: pending.offline_action_id,
              synced_topic_id: pending.synced_topic_id ?? null,
            })),
            joinedTopics: [...topicState.joinedTopics],
            currentTopicId: topicState.currentTopic?.id ?? null,
          };
        },
        syncPendingTopicQueue: async () => {
          const topicStore = useTopicStore.getState();
          const offlineStore = useOfflineStore.getState();
          const pendingBefore = await TauriApi.listPendingTopics();
          const createdTopicIds: string[] = [];
          const pendingNameMap = new Map<string, PendingTopic>();
          pendingBefore.forEach((p) => pendingNameMap.set(p.pending_id, p));

          for (const pending of pendingBefore) {
            try {
              const created = await TauriApi.createTopic({
                name: pending.name,
                description: pending.description ?? '',
                visibility: 'public',
              });
              createdTopicIds.push(created.id);
              pendingNameMap.set(created.id, pending);
              await TauriApi.markPendingTopicSynced(pending.pending_id, created.id);
            } catch (error) {
              errorHandler.log('E2EBridge.syncPendingTopicFailed', error, {
                context: 'registerE2EBridge.syncPendingTopicQueue',
                metadata: { pendingId: pending.pending_id },
              });
            }
          }

          const pendingAfter = await TauriApi.listPendingTopics().catch(() => [] as PendingTopic[]);
          try {
            await topicStore.fetchTopics();
            if (typeof topicStore.refreshPendingTopics === 'function') {
              await topicStore.refreshPendingTopics();
            } else {
              topicStore.setPendingTopics(pendingAfter);
            }
          } catch (error) {
            errorHandler.log('E2EBridge.syncPendingTopicRefreshFailed', error, {
              context: 'registerE2EBridge.syncPendingTopicQueue',
            });
            try {
              topicStore.setPendingTopics(pendingAfter);
            } catch (setError) {
              errorHandler.log('E2EBridge.syncPendingTopicSetFallbackFailed', setError, {
                context: 'registerE2EBridge.syncPendingTopicQueue',
              });
            }
          }

          // 作成済みトピックが一覧に存在しない場合はローカルに追加しておく
          for (const createdId of createdTopicIds) {
            if (!topicStore.topics?.has(createdId)) {
              const source = pendingNameMap.get(createdId);
              topicStore.addTopic({
                id: createdId,
                name: source?.name ?? createdId,
                description: source?.description ?? '',
                createdAt: new Date(),
                memberCount: 0,
                postCount: 0,
                isActive: true,
                tags: [],
                visibility: 'public',
                isJoined: true,
              });
            }
          }

          try {
            offlineStore.clearPendingActions();
            offlineStore.updateLastSyncedAt();
          } catch (error) {
            errorHandler.log('E2EBridge.syncPendingTopicClearOfflineFailed', error, {
              context: 'registerE2EBridge.syncPendingTopicQueue',
            });
          }

          return {
            pendingCountBefore: pendingBefore.length,
            pendingCountAfter: pendingAfter.length,
            createdTopicIds,
          };
        },
        seedTrendingFixture: async (payload: TrendingFixturePayload) => {
          if (!payload || !Array.isArray(payload.topics) || payload.topics.length === 0) {
            throw new Error('Trending fixture topics are required');
          }

          const authStore = useAuthStore.getState();
          const follower = authStore.currentUser;
          const followerNsec = authStore.privateKey;
          if (!follower || !followerNsec) {
            throw new Error('Seeding requires an authenticated user with a private key');
          }

          const authors = new Map<string, { name: string; npub: string; nsec: string }>();
          const createdTopics: Array<{ id: string; name: string; author: string }> = [];
          const topicPosts = new Map<string, Post[]>();
          const topicFixtures = new Map<string, TrendingFixtureTopic>();

          const ensureFollowerSession = async () => {
            // テスト中にキーがロードされていない状態を避けるため、明示的にログインし直す
            await useAuthStore.getState().loginWithNsec(followerNsec, false, {
              name: follower.name,
              displayName: follower.displayName,
              about: follower.about,
              picture: follower.picture,
              publicProfile: follower.publicProfile,
              showOnlineStatus: follower.showOnlineStatus,
            });
            try {
              await SecureStorageApi.addAccount({
                nsec: followerNsec,
                name: follower.name ?? follower.displayName ?? 'e2e',
                display_name: follower.displayName ?? follower.name ?? 'e2e',
                picture: follower.picture ?? undefined,
              });
            } catch (error) {
              errorHandler.log('E2EBridge.seedTrendingFixture.addFollowerAccountFailed', error, {
                context: 'registerE2EBridge.ensureFollowerSession',
                metadata: { npub: follower.npub },
              });
            }
            try {
              await SecureStorageApi.secureLogin(follower.npub);
            } catch (error) {
              errorHandler.log('E2EBridge.seedTrendingFixture.secureLoginFailed', error, {
                context: 'registerE2EBridge.ensureFollowerSession',
                metadata: { npub: follower.npub },
              });
            }
          };

          const ensureAuthorAccount = async (name: string) => {
            const trimmedName = (name || 'author').trim();
            if (authors.has(trimmedName)) {
              return authors.get(trimmedName)!;
            }
            const generated = await useAuthStore.getState().generateNewKeypair(false);
            const current = useAuthStore.getState().currentUser;
            if (!current?.npub || !generated?.nsec) {
              throw new Error('Failed to generate author account for trending fixture');
            }
            try {
              useAuthStore.getState().updateUser({
                name: trimmedName,
                displayName: trimmedName,
              });
            } catch (error) {
              errorHandler.log('E2EBridge.seedTrendingFixture.updateAuthorFailed', error, {
                context: 'registerE2EBridge.seedTrendingFixture.ensureAuthorAccount',
                metadata: { name: trimmedName },
              });
            }
            const record = { name: trimmedName, npub: current.npub, nsec: generated.nsec };
            authors.set(trimmedName, record);
            return record;
          };

          const restoreFollower = async () => {
            await useAuthStore.getState().loginWithNsec(followerNsec, false, {
              name: follower.name,
              displayName: follower.displayName,
              about: follower.about,
              picture: follower.picture,
              publicProfile: follower.publicProfile,
              showOnlineStatus: follower.showOnlineStatus,
            });
            try {
              await SecureStorageApi.secureLogin(follower.npub);
            } catch (error) {
              errorHandler.log(
                'E2EBridge.seedTrendingFixture.secureLoginAfterRestoreFailed',
                error,
                {
                  context: 'registerE2EBridge.seedTrendingFixture.restoreFollower',
                  metadata: { npub: follower.npub },
                },
              );
            }
            try {
              await useTopicStore.getState().fetchTopics();
            } catch (error) {
              errorHandler.log(
                'E2EBridge.seedTrendingFixture.fetchTopicsAfterRestoreFailed',
                error,
                {
                  context: 'registerE2EBridge.seedTrendingFixture.restoreFollower',
                },
              );
            }
          };

          try {
            await ensureFollowerSession();
            for (const topic of payload.topics) {
              const topicName = (topic.title ?? topic.topicId ?? 'trending-topic').trim();
              const primaryAuthorName = (topic.posts?.[0]?.author ?? topicName).trim();
              const primaryAuthor = await ensureAuthorAccount(primaryAuthorName);

              await useAuthStore.getState().loginWithNsec(primaryAuthor.nsec, false, {
                name: primaryAuthor.name,
                displayName: primaryAuthor.name,
              });

              let createdTopicId: string | null = null;
              try {
                const created = await TauriApi.createTopic({
                  name: topicName,
                  description: topic.description ?? '',
                  visibility: 'public',
                });
                createdTopicId = created.id;
                createdTopics.push({
                  id: created.id,
                  name: created.name,
                  author: primaryAuthor.name,
                });
                topicFixtures.set(created.id, topic);
              } catch (error) {
                errorHandler.log('E2EBridge.seedTrendingFixture.createTopicFailed', error, {
                  context: 'registerE2EBridge.seedTrendingFixture.createTopic',
                  metadata: { name: topicName },
                });
                const existing = Array.from(useTopicStore.getState().topics.values()).find(
                  (item) => item.name === topicName,
                );
                if (existing) {
                  createdTopicId = existing.id;
                  createdTopics.push({
                    id: existing.id,
                    name: existing.name,
                    author: primaryAuthor.name,
                  });
                  topicFixtures.set(existing.id, topic);
                } else {
                  throw error;
                }
              }

              if (!createdTopicId) {
                continue;
              }

              for (const post of topic.posts ?? []) {
                const authorName = (post.author ?? primaryAuthor.name).trim();
                const authorAccount = await ensureAuthorAccount(authorName);
                if (useAuthStore.getState().currentUser?.npub !== authorAccount.npub) {
                  await useAuthStore.getState().loginWithNsec(authorAccount.nsec, false, {
                    name: authorAccount.name,
                    displayName: authorAccount.name,
                  });
                }
                const content = post.title || post.id || 'Trending post';
                try {
                  const created = await TauriApi.createPost({
                    content,
                    topic_id: createdTopicId,
                    thread_uuid: uuidv4(),
                  });
                  const mapped = await mapPostResponseToDomain(created);
                  mapped.content = content;
                  const enrichedAuthor = `${authorAccount.name} ${content}`.trim();
                  mapped.author = {
                    ...mapped.author,
                    name: enrichedAuthor,
                    displayName: enrichedAuthor,
                  };
                  const posts = topicPosts.get(createdTopicId) ?? [];
                  posts.push(mapped);
                  topicPosts.set(createdTopicId, posts);
                } catch (error) {
                  errorHandler.log('E2EBridge.seedTrendingFixture.createPostFailed', error, {
                    context: 'registerE2EBridge.seedTrendingFixture.createPost',
                    metadata: { topicId: createdTopicId, author: authorAccount.npub },
                  });
                }
              }
            }
          } finally {
            await restoreFollower();
          }

          for (const author of authors.values()) {
            if (author.npub === follower.npub) {
              continue;
            }
            try {
              await TauriApi.followUser(follower.npub, author.npub);
            } catch (error) {
              errorHandler.log('E2EBridge.seedTrendingFixture.followFailed', error, {
                context: 'registerE2EBridge.seedTrendingFixture.follow',
                metadata: { follower: follower.npub, target: author.npub },
              });
            }
          }

          try {
            const now = Date.now();
            const trendingTopics: TrendingTopicsResult = {
              generatedAt: now,
              topics: createdTopics.map((topic, index) => {
                const posts = topicPosts.get(topic.id) ?? [];
                const fixtureTopic = topicFixtures.get(topic.id);
                return {
                  topicId: topic.id,
                  name: topic.name,
                  description: fixtureTopic?.description ?? '',
                  memberCount: Math.max(1, posts.length > 0 ? 2 : 1),
                  postCount: posts.length,
                  trendingScore: Math.max(1, posts.length * 10 - index),
                  rank: index + 1,
                  scoreChange: null,
                };
              }),
            };

            const topicIds = trendingTopics.topics.map((topic) => topic.topicId);
            const trendingPosts: TrendingPostsResult = {
              generatedAt: now,
              topics: createdTopics.map((topic, index) => ({
                topicId: topic.id,
                topicName: topic.name,
                relativeRank: index + 1,
                posts: topicPosts.get(topic.id) ?? [],
              })),
            };

            const followingPage: FollowingFeedPageResult = {
              cursor: null,
              items: Array.from(topicPosts.values()).flat(),
              nextCursor: null,
              hasMore: false,
              serverTime: now,
            };

            queryClient.setQueryData(trendingTopicsQueryKey(10), trendingTopics, {
              updatedAt: now,
            });
            queryClient.setQueriesData({ queryKey: ['trending', 'topics'] }, () => trendingTopics);
            queryClient.setQueryDefaults(trendingTopicsQueryKey(10), {
              staleTime: 10 * 60 * 1000,
              gcTime: 15 * 60 * 1000,
              refetchOnMount: false,
              refetchOnReconnect: false,
              refetchInterval: false,
              queryFn: async () => trendingTopics,
            });
            queryClient.setQueryData(trendingPostsQueryKey(topicIds, 3), trendingPosts, {
              updatedAt: now,
            });
            queryClient.setQueriesData({ queryKey: ['trending', 'posts'] }, () => trendingPosts);
            queryClient.setQueryDefaults(trendingPostsQueryKey(topicIds, 3), {
              staleTime: 10 * 60 * 1000,
              gcTime: 15 * 60 * 1000,
              refetchOnMount: false,
              refetchOnReconnect: false,
              refetchInterval: false,
              queryFn: async () => trendingPosts,
            });
            const hydrateFollowingFeedCache = (limit: number, includeReactions: boolean) => {
              const key = followingFeedQueryKey(limit, includeReactions);
              queryClient.setQueryData(
                key,
                { pages: [followingPage], pageParams: [null] },
                { updatedAt: now },
              );
              queryClient.setQueryDefaults(key, {
                staleTime: 10 * 60 * 1000,
                gcTime: 15 * 60 * 1000,
                refetchOnMount: false,
                refetchOnReconnect: false,
                refetchInterval: false,
                queryFn: async () => ({ pages: [followingPage], pageParams: [null] }),
              });
            };
            hydrateFollowingFeedCache(20, false);
            hydrateFollowingFeedCache(20, true);
            hydrateFollowingFeedCache(10, true);
            hydrateFollowingFeedCache(10, false);
            queryClient.setQueriesData({ queryKey: ['followingFeed'] }, () => ({
              pages: [followingPage],
              pageParams: [null],
            }));
          } catch (error) {
            errorHandler.log('E2EBridge.seedTrendingFixture.cacheFailed', error, {
              context: 'registerE2EBridge.seedTrendingFixture.cache',
            });
          }

          return {
            topics: createdTopics,
            authors: Array.from(authors.values()).map(({ name, npub }) => ({ name, npub })),
            followerNpub: follower.npub,
          };
        },
        seedUserSearchFixture: async (payload: { users: UserSearchFixtureUser[] }) => {
          if (!payload || !Array.isArray(payload.users) || payload.users.length === 0) {
            throw new Error('User search fixtures are required');
          }
          const authStore = useAuthStore.getState();
          const baseUser = authStore.currentUser;
          const baseNsec = authStore.privateKey;
          if (!baseUser || !baseNsec) {
            throw new Error('Seeding requires an authenticated user with a private key');
          }

          const baseProfile = {
            name: baseUser.name,
            displayName: baseUser.displayName,
            about: baseUser.about,
            picture: baseUser.picture,
            publicProfile: baseUser.publicProfile,
            showOnlineStatus: baseUser.showOnlineStatus,
          };

          const seeded: Array<{
            npub: string;
            displayName: string;
            about: string;
            follow: boolean;
          }> = [];
          const profiles: UserProfile[] = [];

          for (const [index, entry] of payload.users.entries()) {
            const displayName = (entry.displayName || `search-user-${index + 1}`).trim();
            const about = entry.about ?? '';
            try {
              await useAuthStore.getState().generateNewKeypair(false);
              const fixtureUser = useAuthStore.getState().currentUser;
              if (!fixtureUser?.npub) {
                throw new Error('Failed to generate fixture account');
              }
              useAuthStore.getState().updateUser({
                name: displayName,
                displayName,
                about,
                publicProfile: true,
                showOnlineStatus: true,
              });
              try {
                await NostrAPI.updateMetadata({
                  name: displayName,
                  display_name: displayName,
                  about,
                  kukuri_privacy: { public_profile: true, show_online_status: true },
                });
              } catch (error) {
                errorHandler.log('E2EBridge.userSearchSeedMetadataFailed', error, {
                  context: 'registerE2EBridge.seedUserSearchFixture',
                  metadata: { npub: fixtureUser.npub },
                });
              }
              seeded.push({
                npub: fixtureUser.npub,
                displayName,
                about,
                follow: Boolean(entry.follow),
              });
              profiles.push({
                npub: fixtureUser.npub,
                pubkey: fixtureUser.pubkey ?? fixtureUser.npub,
                name: displayName,
                display_name: displayName,
                about,
                picture: fixtureUser.picture ?? null,
                banner: null,
                website: null,
                nip05: null,
                is_profile_public: true,
                show_online_status: true,
              });
            } catch (error) {
              errorHandler.log('E2EBridge.seedUserSearchFixtureFailed', error, {
                context: 'registerE2EBridge.seedUserSearchFixture',
                metadata: { index },
              });
            }
          }

          try {
            await useAuthStore.getState().loginWithNsec(baseNsec, false, {
              name: baseProfile.name,
              displayName: baseProfile.displayName,
              about: baseProfile.about,
              picture: baseProfile.picture,
              publicProfile: baseProfile.publicProfile,
              showOnlineStatus: baseProfile.showOnlineStatus,
            });
            try {
              await SecureStorageApi.secureLogin(baseUser.npub);
            } catch (error) {
              errorHandler.log('E2EBridge.userSearchSeedSecureLoginFailed', error, {
                context: 'registerE2EBridge.seedUserSearchFixture.restore',
                metadata: { npub: baseUser.npub },
              });
            }
            try {
              await useAuthStore.getState().loadAccounts();
            } catch (error) {
              errorHandler.log('E2EBridge.userSearchSeedLoadAccountsFailed', error, {
                context: 'registerE2EBridge.seedUserSearchFixture.restore',
              });
            }
          } catch (error) {
            errorHandler.log('E2EBridge.seedUserSearchFixtureRestoreFailed', error, {
              context: 'registerE2EBridge.seedUserSearchFixture',
            });
          }

          for (const entry of seeded.filter((user) => user.follow)) {
            try {
              await TauriApi.followUser(baseUser.npub, entry.npub);
            } catch (error) {
              errorHandler.log('E2EBridge.userSearchSeedFollowFailed', error, {
                context: 'registerE2EBridge.seedUserSearchFixture',
                metadata: { target: entry.npub },
              });
            }
          }

          return {
            users: seeded.map(({ follow, ...user }) => ({
              ...user,
              isFollowed: follow,
            })),
          };
        },
        primeUserSearchRateLimit: async (payload?: { query?: string; limit?: number }) => {
          const authState = useAuthStore.getState();
          const viewerNpub = authState.currentUser?.npub ?? null;
          const query =
            (payload?.query?.trim().length ?? 0) > 0
              ? (payload?.query?.trim() ?? '')
              : 'rate-limit';
          const limit = payload?.limit && payload.limit > 0 ? payload.limit : 40;
          let attempts = 0;
          let retryAfterSeconds: number | null = null;

          while (attempts < limit) {
            attempts += 1;
            try {
              await TauriApi.searchUsers({
                query,
                cursor: null,
                limit: 1,
                sort: 'relevance',
                allowIncomplete: true,
                viewerNpub,
              });
            } catch (error) {
              const code =
                error instanceof TauriCommandError
                  ? error.code
                  : ((error as { code?: string | null })?.code ?? null);
              const rateLimitedDetails =
                (error as {
                  RateLimited?: { retry_after_seconds?: number | string | null };
                  rateLimited?: { retry_after_seconds?: number | string | null };
                  retry_after_seconds?: number | string | null;
                }) ?? null;

              if (code === 'RATE_LIMITED') {
                const retrySecondsFromDetails =
                  Number(
                    error instanceof TauriCommandError ? error.details?.retry_after_seconds : null,
                  ) || null;
                retryAfterSeconds = retrySecondsFromDetails;
                if (retryAfterSeconds === null) {
                  retryAfterSeconds = 5;
                }
                break;
              }

              const retrySecondsFromPayload =
                rateLimitedDetails?.RateLimited?.retry_after_seconds ??
                rateLimitedDetails?.rateLimited?.retry_after_seconds ??
                rateLimitedDetails?.retry_after_seconds ??
                null;

              if (retrySecondsFromPayload !== null && retrySecondsFromPayload !== undefined) {
                retryAfterSeconds = Number(retrySecondsFromPayload) || null;
                retryAfterSeconds =
                  retryAfterSeconds !== null && Number.isFinite(retryAfterSeconds)
                    ? retryAfterSeconds
                    : null;
                if (retryAfterSeconds === null) {
                  retryAfterSeconds = 5;
                }
                break;
              }
              throw error;
            }
          }

          return {
            attempts,
            retryAfterSeconds,
            triggered: retryAfterSeconds !== null,
          };
        },
        getBootstrapSnapshot: async () => {
          const config = await p2pApi.getBootstrapConfig();
          return bootstrapSnapshotFromConfig(config);
        },
        applyCliBootstrap: async () => {
          const config = await p2pApi.applyCliBootstrapNodes();
          await refreshRelayStatusSafe();
          return bootstrapSnapshotFromConfig(config);
        },
        clearBootstrapNodes: async () => {
          await p2pApi.clearBootstrapNodes();
          await refreshRelayStatusSafe();
          const config = await p2pApi.getBootstrapConfig();
          return bootstrapSnapshotFromConfig(config);
        },
        seedFriendPlusAccounts: async () => {
          const authState = useAuthStore.getState();
          const requesterUser = authState.currentUser;
          const requesterNsec = authState.privateKey;
          if (!requesterUser?.npub || !requesterUser.pubkey || !requesterNsec) {
            throw new Error('Authenticated requester account is required');
          }

          const runId = Date.now().toString(36);
          const requesterEntry = {
            npub: requesterUser.npub,
            pubkey: requesterUser.pubkey,
            nsec: requesterNsec,
            name:
              requesterUser.displayName ?? requesterUser.name ?? `friend-plus-requester-${runId}`,
            about: requesterUser.about ?? '',
            picture: requesterUser.picture ?? undefined,
            publicProfile: requesterUser.publicProfile ?? true,
            showOnlineStatus: requesterUser.showOnlineStatus ?? true,
          };

          const createGeneratedAccount = async (label: string, suffix: string) => {
            await useAuthStore.getState().generateNewKeypair(false);
            const current = useAuthStore.getState().currentUser;
            const nsec = useAuthStore.getState().privateKey;
            if (!current?.npub || !current.pubkey || !nsec) {
              throw new Error(`Failed to generate ${label} account`);
            }
            const profileName = `${label}-${runId}-${suffix}`;
            useAuthStore.getState().updateUser({
              name: profileName,
              displayName: profileName,
              about: `E2E friend_plus ${label}`,
              publicProfile: true,
              showOnlineStatus: true,
            });
            return {
              npub: current.npub,
              pubkey: current.pubkey,
              nsec,
              name: profileName,
              about: `E2E friend_plus ${label}`,
              picture: current.picture ?? undefined,
              publicProfile: true,
              showOnlineStatus: true,
            };
          };

          const inviterEntry = await createGeneratedAccount('friend-plus-inviter', '1');
          const friendEntry = await createGeneratedAccount('friend-plus-friend', '2');

          const restoreAccount = async (entry: {
            npub: string;
            nsec: string;
            name: string;
            about: string;
            picture?: string;
            publicProfile: boolean;
            showOnlineStatus: boolean;
          }) => {
            await useAuthStore.getState().loginWithNsec(entry.nsec, false, {
              name: entry.name,
              displayName: entry.name,
              about: entry.about,
              picture: entry.picture,
              publicProfile: entry.publicProfile,
              showOnlineStatus: entry.showOnlineStatus,
            });
            try {
              await SecureStorageApi.secureLogin(entry.npub);
            } catch (error) {
              errorHandler.log('E2EBridge.seedFriendPlusAccounts.secureLoginFailed', error, {
                context: 'registerE2EBridge.seedFriendPlusAccounts.restoreAccount',
                metadata: { npub: entry.npub },
              });
            }
          };

          const follow = async (follower: typeof requesterEntry, target: typeof requesterEntry) => {
            await restoreAccount(follower);
            await TauriApi.followUser(follower.npub, target.npub);
          };

          await follow(inviterEntry, friendEntry);
          await follow(friendEntry, inviterEntry);
          await follow(requesterEntry, friendEntry);
          await follow(friendEntry, requesterEntry);

          await restoreAccount(requesterEntry);
          try {
            await useAuthStore.getState().loadAccounts();
          } catch (error) {
            errorHandler.log('E2EBridge.seedFriendPlusAccounts.loadAccountsFailed', error, {
              context: 'registerE2EBridge.seedFriendPlusAccounts',
            });
          }

          return {
            requester: {
              npub: requesterEntry.npub,
              pubkey: requesterEntry.pubkey,
            },
            inviter: {
              npub: inviterEntry.npub,
              pubkey: inviterEntry.pubkey,
            },
            friend: {
              npub: friendEntry.npub,
              pubkey: friendEntry.pubkey,
            },
          };
        },
        accessControlRequestJoin: async (payload: AccessControlRequestJoinPayload) => {
          return await accessControlApi.requestJoin({
            topic_id: payload?.topic_id,
            scope: payload?.scope,
            invite_event_json: payload?.invite_event_json,
            target_pubkey: payload?.target_pubkey,
            broadcast_to_topic: payload?.broadcast_to_topic,
          });
        },
        accessControlListJoinRequests: async () => {
          return await accessControlApi.listJoinRequests();
        },
        accessControlApproveJoinRequest: async (payload: { event_id: string }) => {
          const eventId = payload?.event_id?.trim();
          if (!eventId) {
            throw new Error('event_id is required to approve join.request');
          }
          return await accessControlApi.approveJoinRequest({ event_id: eventId });
        },
        accessControlIngestEventJson: async (payload: AccessControlIngestEventPayload) => {
          if (!payload || payload.event_json === null || payload.event_json === undefined) {
            throw new Error('event_json is required');
          }
          await accessControlApi.ingestEventJson({
            event_json: payload.event_json,
          });
        },
        communityNodeAuthFlow: async (payload: { baseUrl: string }) => {
          const baseUrl = payload?.baseUrl?.trim();
          if (!baseUrl) {
            throw new Error('baseUrl is required to authenticate against community node');
          }
          try {
            const normalized = baseUrl.replace(/\/+$/, '');
            const currentConfig = await communityNodeApi.getConfig();
            const nodes = currentConfig?.nodes ?? [];
            const hasNode = nodes.some((node) => node.base_url === normalized);
            if (!hasNode) {
              await communityNodeApi.setConfig([
                ...nodes.map((node) => ({ base_url: node.base_url, roles: node.roles })),
                { base_url: normalized, roles: defaultCommunityNodeRoles },
              ]);
            }
            const auth = await communityNodeApi.authenticate(normalized);
            const config = await communityNodeApi.getConfig();
            const consents = await communityNodeApi.getConsentStatus(normalized);
            await queryClient.invalidateQueries({ queryKey: ['community-node', 'config'] });
            await queryClient.invalidateQueries({ queryKey: ['community-node', 'group-keys'] });
            await queryClient.invalidateQueries({ queryKey: ['community-node', 'consents'] });
            return { config, auth, consents };
          } catch (error) {
            errorHandler.log('E2EBridge.communityNodeAuthFlowFailed', error, {
              context: 'registerE2EBridge.communityNodeAuthFlow',
              metadata: { baseUrl },
            });
            throw error;
          }
        },
        communityNodeListGroupKeys: async () => {
          try {
            return await communityNodeApi.listGroupKeys();
          } catch (error) {
            errorHandler.log('E2EBridge.communityNodeListGroupKeysFailed', error, {
              context: 'registerE2EBridge.communityNodeListGroupKeys',
            });
            throw error;
          }
        },
        communityNodeListBootstrapNodes: async () => {
          try {
            return await communityNodeApi.listBootstrapNodes();
          } catch (error) {
            errorHandler.log('E2EBridge.communityNodeListBootstrapNodesFailed', error, {
              context: 'registerE2EBridge.communityNodeListBootstrapNodes',
            });
            throw error;
          }
        },
        communityNodeListBootstrapServices: async (payload: { topicId: string }) => {
          const topicId = payload?.topicId?.trim();
          if (!topicId) {
            throw new Error('topicId is required to list bootstrap services');
          }
          try {
            return await communityNodeApi.listBootstrapServices(topicId);
          } catch (error) {
            errorHandler.log('E2EBridge.communityNodeListBootstrapServicesFailed', error, {
              context: 'registerE2EBridge.communityNodeListBootstrapServices',
              metadata: { topicId },
            });
            throw error;
          }
        },
      };
    }

    window.__KUKURI_E2E_BOOTSTRAP__ = registerE2EBridge;
    setupMessageBridge();
    const domReady = setupDomBridge();

    if (!domReady) {
      setE2EStatus('error');
      errorHandler.log('E2EBridge.domBridgeUnavailable', new Error('DOM bridge host unavailable'), {
        context: 'registerE2EBridge.setupDomBridge',
      });
      return;
    }

    setE2EStatus('registered');
  } catch (error) {
    setE2EStatus('error');
    errorHandler.log('E2EBridge.registerFailed', error, {
      context: 'registerE2EBridge.register',
    });
  }
}

function setupMessageBridge(): void {
  if (typeof window === 'undefined' || window.__KUKURI_E2E_MESSAGE_HANDLER__) {
    return;
  }

  window.addEventListener('message', async (event: MessageEvent) => {
    const payload = event.data as BridgeRequest & { type?: string };
    if (!payload || payload.type !== 'KUKURI_E2E_CALL' || !payload.requestId) {
      return;
    }
    const response = await executeBridgeRequest(payload);
    window.postMessage({ type: 'KUKURI_E2E_RESPONSE', ...response }, '*');
  });

  window.__KUKURI_E2E_MESSAGE_HANDLER__ = true;
}

function setupDomBridge(): boolean {
  if (typeof document === 'undefined' || document.__KUKURI_E2E_DOM_BRIDGE__) {
    return true;
  }

  const ensureChannelElement = (): HTMLElement | null => {
    const host = document.body ?? document.documentElement;
    if (!host) {
      return null;
    }
    const existing = document.getElementById(CHANNEL_ID);
    if (existing) {
      return existing;
    }
    const channel = document.createElement('div');
    channel.id = CHANNEL_ID;
    channel.style.display = 'none';
    host.appendChild(channel);
    return channel;
  };

  const channel = ensureChannelElement();
  if (!channel) {
    return false;
  }

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (mutation.type !== 'attributes' || mutation.attributeName !== REQUEST_ATTR) {
        continue;
      }
      const rawPayload = channel.getAttribute(REQUEST_ATTR);
      if (!rawPayload) {
        continue;
      }
      channel.setAttribute(REQUEST_ATTR, '');

      let parsed: BridgeRequest | null = null;
      try {
        parsed = JSON.parse(rawPayload) as BridgeRequest;
      } catch (error) {
        errorHandler.log('E2EBridge.domBridgeInvalidPayload', error, {
          context: 'registerE2EBridge.domBridge.parse',
          metadata: { rawPayload },
        });
      }

      if (parsed) {
        void handleBridgeRequest(parsed, channel);
      }
    }
  });

  observer.observe(channel, { attributes: true, attributeFilter: [REQUEST_ATTR] });
  channel.setAttribute(RESPONSE_ATTR, '');
  channel.setAttribute(READY_ATTR, '1');
  document.__KUKURI_E2E_DOM_BRIDGE__ = true;
  return true;
}

async function handleBridgeRequest(request: BridgeRequest, channel: HTMLElement): Promise<void> {
  const response = await executeBridgeRequest(request);
  channel.setAttribute(RESPONSE_ATTR, JSON.stringify(response));
}

async function executeBridgeRequest(request: BridgeRequest): Promise<BridgeResponse> {
  const helper = typeof window === 'undefined' ? undefined : window.__KUKURI_E2E__;
  if (!helper) {
    return {
      requestId: request.requestId,
      error: 'E2E bridge is unavailable',
    };
  }

  const handler = helper[request.action];
  if (typeof handler !== 'function') {
    return {
      requestId: request.requestId,
      error: `Unknown bridge action: ${request.action}`,
    };
  }

  try {
    const result = await handler(request.args as never);
    return {
      requestId: request.requestId,
      result: result ?? null,
    };
  } catch (error) {
    return {
      requestId: request.requestId,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

if (typeof window !== 'undefined' && !window.__KUKURI_E2E_BOOTSTRAP__) {
  window.__KUKURI_E2E_BOOTSTRAP__ = registerE2EBridge;
}
