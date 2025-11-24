import { SecureStorageApi } from '@/lib/api/secureStorage';
import { TauriApi, type SeedDirectMessageConversationResult } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { persistKeys } from '@/stores/config/persist';
import {
  clearFallbackAccounts,
  listFallbackAccountMetadata,
  useAuthStore,
} from '@/stores/authStore';
import { mapApiMessageToModel, useDirectMessageStore } from '@/stores/directMessageStore';
import { useOfflineStore } from '@/stores/offlineStore';
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
