import { SecureStorageApi } from '@/lib/api/secureStorage';
import {
  TauriApi,
  type SeedDirectMessageConversationResult,
  type PendingTopic,
} from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
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
import type { Post } from '@/stores';
import { useComposerStore } from '@/stores/composerStore';
import { mapApiMessageToModel, useDirectMessageStore } from '@/stores/directMessageStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { useTopicStore } from '@/stores/topicStore';
import { getE2EStatus, setE2EStatus, type E2EStatus } from './e2eStatus';

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

interface ProfileAvatarFixture {
  base64: string;
  format: string;
  fileName?: string;
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

export interface E2EBridge {
  resetAppState: () => Promise<void>;
  getAuthSnapshot: () => AuthSnapshot;
  getOfflineSnapshot: () => OfflineSnapshot;
  getDirectMessageSnapshot: () => DirectMessageSnapshot;
  setProfileAvatarFixture: (fixture: ProfileAvatarFixture | null) => void;
  consumeProfileAvatarFixture: () => ProfileAvatarFixture | null;
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
  persistKeys.privacy,
  persistKeys.keyManagement,
];

const CHANNEL_ID = 'kukuri-e2e-channel';
const REQUEST_ATTR = 'data-e2e-request';
const RESPONSE_ATTR = 'data-e2e-response';
const READY_ATTR = 'data-e2e-ready';

type BridgeRequest = { requestId: string; action: keyof E2EBridge; args?: unknown };
type BridgeResponse = { requestId: string; result?: unknown; error?: string };

let pendingAvatarFixture: ProfileAvatarFixture | null = null;

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
          if (typeof window !== 'undefined') {
            (
              window as unknown as { __E2E_KEEP_LOCAL_TOPICS__?: boolean }
            ).__E2E_KEEP_LOCAL_TOPICS__ = false;
            (
              window as unknown as { __E2E_PENDING_TOPICS__?: PendingTopic[] }
            ).__E2E_PENDING_TOPICS__ = [];
            (
              window as unknown as { __E2E_DELETED_TOPIC_IDS__?: string[] }
            ).__E2E_DELETED_TOPIC_IDS__ = [];
          }
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
        setProfileAvatarFixture: (fixture: ProfileAvatarFixture | null) => {
          pendingAvatarFixture = fixture ?? null;
        },
        consumeProfileAvatarFixture: () => {
          const fixture = pendingAvatarFixture;
          pendingAvatarFixture = null;
          return fixture;
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
          const e2ePending =
            (typeof window !== 'undefined' &&
              (window as unknown as { __E2E_PENDING_TOPICS__?: PendingTopic[] })
                .__E2E_PENDING_TOPICS__) ||
            [];

          // E2Eオフライン強制時はローカルストアのみで完結させ、Tauriコマンドを呼ばない
          if (e2ePending.length > 0) {
            const createdIds: string[] = [];
            let persistedToBackend = false;
            for (const pending of e2ePending) {
              let createdId = pending.pending_id;
              try {
                const created = await TauriApi.createTopic({
                  name: pending.name,
                  description: pending.description ?? '',
                  visibility: 'public',
                });
                createdId = created.id;
                persistedToBackend = true;
                topicStore.addTopic({
                  id: created.id,
                  name: created.name,
                  description: created.description ?? '',
                  createdAt: new Date(created.created_at * 1000),
                  memberCount: created.member_count ?? 0,
                  postCount: created.post_count ?? 0,
                  isActive: true,
                  tags: [],
                  visibility: created.visibility ?? 'public',
                  isJoined: created.is_joined ?? true,
                });
              } catch (error) {
                errorHandler.log('E2EBridge.syncPendingTopicCreateFallback', error, {
                  context: 'registerE2EBridge.syncPendingTopicQueue.e2e',
                  metadata: { pendingId: pending.pending_id },
                });
                topicStore.addTopic({
                  id: createdId,
                  name: pending.name,
                  description: pending.description ?? '',
                  createdAt: new Date(),
                  memberCount: 0,
                  postCount: 0,
                  isActive: true,
                  tags: [],
                  visibility: 'public',
                  isJoined: true,
                });
              }
              createdIds.push(createdId);
              try {
                useComposerStore.getState().resolvePendingTopic(pending.pending_id, createdId);
              } catch (error) {
                errorHandler.log('E2EBridge.syncPendingTopicResolveComposerFailed', error, {
                  context: 'registerE2EBridge.syncPendingTopicQueue.e2e',
                  metadata: { pendingId: pending.pending_id },
                });
              }
              topicStore.removePendingTopic(pending.pending_id);
            }
            topicStore.setPendingTopics([]);
            (
              window as unknown as { __E2E_PENDING_TOPICS__?: PendingTopic[] }
            ).__E2E_PENDING_TOPICS__ = [];
            if (persistedToBackend) {
              try {
                await topicStore.fetchTopics();
              } catch (error) {
                errorHandler.log('E2EBridge.syncPendingTopicFetchAfterE2E', error, {
                  context: 'registerE2EBridge.syncPendingTopicQueue.e2e',
                });
              }
            }
            try {
              offlineStore.clearPendingActions();
              offlineStore.updateLastSyncedAt();
            } catch (error) {
              errorHandler.log('E2EBridge.syncPendingTopicClearOfflineFailed', error, {
                context: 'registerE2EBridge.syncPendingTopicQueue.e2e',
              });
            }
            return {
              pendingCountBefore: e2ePending.length,
              pendingCountAfter: 0,
              createdTopicIds: createdIds,
            };
          }

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

          let pendingAfter: PendingTopic[] = [];
          try {
            pendingAfter = await TauriApi.listPendingTopics();
          } catch {
            pendingAfter = [];
          }
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
