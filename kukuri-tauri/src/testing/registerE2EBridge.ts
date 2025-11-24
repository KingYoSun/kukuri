import { SecureStorageApi } from '@/lib/api/secureStorage';
import {
  TauriApi,
  type SeedDirectMessageConversationResult,
  type PendingTopic,
} from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { persistKeys } from '@/stores/config/persist';
import {
  clearFallbackAccounts,
  listFallbackAccountMetadata,
  useAuthStore,
} from '@/stores/authStore';
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
