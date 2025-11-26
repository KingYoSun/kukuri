import { browser } from '@wdio/globals';
import type { SeedDirectMessageConversationResult } from '@/lib/api/tauri';
import type { E2EBridge } from '@/testing/registerE2EBridge';
import { waitForAppReady } from './waitForAppReady';

const CHANNEL_ID = 'kukuri-e2e-channel';
const REQUEST_ATTR = 'data-e2e-request';
const RESPONSE_ATTR = 'data-e2e-response';
const READY_ATTR = 'data-e2e-ready';
// CI 環境では起動が重くなるためタイムアウトをやや長めに取る
const BRIDGE_TIMEOUT_MS = 20000;

export type BridgeAction =
  | 'resetAppState'
  | 'getAuthSnapshot'
  | 'getOfflineSnapshot'
  | 'getDirectMessageSnapshot'
  | 'setProfileAvatarFixture'
  | 'consumeProfileAvatarFixture'
  | 'switchAccount'
  | 'seedDirectMessageConversation'
  | 'getTopicSnapshot'
  | 'syncPendingTopicQueue'
  | 'seedTrendingFixture'
  | 'seedUserSearchFixture'
  | 'primeUserSearchRateLimit';

export interface AuthSnapshot {
  currentUser: {
    npub: string | null;
    displayName?: string | null;
    publicProfile?: boolean;
    showOnlineStatus?: boolean;
    picture?: string | null;
  } | null;
  accounts: Array<{
    npub: string;
    display_name: string;
    name?: string;
    pubkey?: string;
    picture?: string;
    last_used?: string;
    public_profile?: boolean;
    show_online_status?: boolean;
  }>;
  isAuthenticated: boolean;
  hasPrivateKey: boolean;
  fallbackAccounts: Array<{
    npub: string;
    display_name: string;
    name?: string;
    pubkey?: string;
    picture?: string;
    last_used?: string;
    public_profile?: boolean;
    show_online_status?: boolean;
  }>;
}

export interface OfflineSnapshot {
  isOnline: boolean;
  isSyncing: boolean;
  lastSyncedAt: number | null;
  pendingActionCount: number;
}

export interface DirectMessageSnapshot {
  unreadCounts: Record<string, number>;
  unreadTotal: number;
  conversations: Record<string, number>;
  conversationKeys: string[];
  latestConversationNpub: string | null;
  activeConversationNpub: string | null;
  isInboxOpen: boolean;
  isDialogOpen: boolean;
}

export interface TopicSnapshot {
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
}

export interface SyncPendingTopicResult {
  pendingCountBefore: number;
  pendingCountAfter: number;
  createdTopicIds: string[];
}

export interface TrendingFixturePost {
  id?: string;
  title: string;
  author?: string;
}

export interface TrendingFixtureTopic {
  topicId?: string;
  title: string;
  description?: string;
  posts?: TrendingFixturePost[];
}

export interface TrendingFixture {
  topics: TrendingFixtureTopic[];
}

export interface SeedTrendingFixtureResult {
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

export interface AvatarFixture {
  base64: string;
  format: string;
  fileName?: string;
}

export interface UserSearchFixtureUser {
  displayName: string;
  about?: string;
  follow?: boolean;
}

export interface SeedUserSearchFixtureResult {
  users: Array<{
    npub: string;
    displayName: string;
    about: string;
    isFollowed: boolean;
  }>;
}

export interface PrimeUserSearchRateLimitResult {
  attempts: number;
  retryAfterSeconds: number | null;
  triggered: boolean;
}

type BridgeResultMap = {
  resetAppState: null;
  getAuthSnapshot: AuthSnapshot;
  getOfflineSnapshot: OfflineSnapshot;
  getDirectMessageSnapshot: DirectMessageSnapshot;
  setProfileAvatarFixture: null;
  consumeProfileAvatarFixture: AvatarFixture | null;
  switchAccount: null;
  seedDirectMessageConversation: SeedDirectMessageConversationResult;
  getTopicSnapshot: TopicSnapshot;
  syncPendingTopicQueue: SyncPendingTopicResult;
  seedTrendingFixture: SeedTrendingFixtureResult;
  seedUserSearchFixture: SeedUserSearchFixtureResult;
  primeUserSearchRateLimit: PrimeUserSearchRateLimitResult;
};

declare global {
  interface Window {
    __KUKURI_E2E__?: E2EBridge;
    __KUKURI_E2E_BOOTSTRAP__?: () => Promise<void> | void;
  }
}

const serializeError = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  if (error && typeof error === 'object') {
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }
  return String(error);
};

export async function callBridge<T extends BridgeAction>(
  action: T,
  payload?: unknown,
): Promise<BridgeResultMap[T]> {
  const response = await browser.executeAsync<
    { error?: string; result?: unknown },
    [
      BridgeAction,
      unknown,
      {
        channelId: string;
        requestAttr: string;
        responseAttr: string;
        readyAttr: string;
        timeoutMs: number;
      },
    ]
  >(
    async (name, args, config, done) => {
      const { channelId, requestAttr, responseAttr, readyAttr, timeoutMs } = config;
      const delay = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
      const toMessage = (error: unknown): string => {
        if (error instanceof Error) {
          return error.message;
        }
        if (error && typeof error === 'object') {
          try {
            return JSON.stringify(error);
          } catch {
            return String(error);
          }
        }
        return String(error);
      };

      const runDirect = async () => {
        const helper = window.__KUKURI_E2E__;
        if (!helper) {
          return null;
        }
        const fn = helper[name];
        if (typeof fn !== 'function') {
          return { error: `Unknown bridge action: ${name}` };
        }
        try {
          const result = await (args !== undefined ? fn(args as never) : fn());
          return { result: result ?? null };
        } catch (error) {
          return { error: toMessage(error) };
        }
      };

      const direct = await runDirect();
      if (direct) {
        done(direct);
        return;
      }

      if (typeof window.__KUKURI_E2E_BOOTSTRAP__ === 'function') {
        try {
          await window.__KUKURI_E2E_BOOTSTRAP__();
          const retried = await runDirect();
          if (retried) {
            done(retried);
            return;
          }
        } catch {
          // Bootstrap failures are handled by the DOM bridge below.
        }
      }

      const waitForBridgeReady = async () => {
        const deadline = Date.now() + timeoutMs;
        while (Date.now() < deadline) {
          if (window.__KUKURI_E2E__) {
            return 'helper' as const;
          }
          const channelCandidate = document.getElementById(channelId);
          if (channelCandidate && channelCandidate.getAttribute(readyAttr) === '1') {
            return 'channel' as const;
          }
          await delay(50);
        }
        return null;
      };

      const readyTarget = await waitForBridgeReady();
      if (readyTarget === 'helper') {
        const directAfterReady = await runDirect();
        if (directAfterReady) {
          done(directAfterReady);
          return;
        }
      }

      const channelStatus = (window as Record<string, unknown>).__KUKURI_E2E_STATUS__ ?? 'unknown';
      const channel = document.getElementById(channelId);
      const domBridgeReady =
        (document as Document & { __KUKURI_E2E_DOM_BRIDGE__?: boolean })
          .__KUKURI_E2E_DOM_BRIDGE__ ?? false;
      const readyValue = channel?.getAttribute(readyAttr);
      if (!channel || readyValue !== '1') {
        const detail = [
          `status=${String(channelStatus)}`,
          `channel=${channel ? 'found' : 'missing'}`,
          `ready=${readyValue ?? 'none'}`,
          `domBridge=${String(domBridgeReady)}`,
        ].join(', ');
        done({ error: `E2E channel is unavailable (${detail})` });
        return;
      }

      const requestId =
        typeof crypto !== 'undefined' && 'randomUUID' in crypto
          ? crypto.randomUUID()
          : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
      const requestPayload = JSON.stringify({ requestId, action: name, args });

      let settled = false;
      const finish = (result: { error?: string; result?: unknown }) => {
        if (settled) {
          return;
        }
        settled = true;
        observer.disconnect();
        window.clearTimeout(timeoutId);
        done(result);
      };

      const observer = new MutationObserver((mutations) => {
        for (const mutation of mutations) {
          if (mutation.type !== 'attributes' || mutation.attributeName !== responseAttr) {
            continue;
          }
          const raw = channel.getAttribute(responseAttr);
          if (!raw) {
            continue;
          }
          try {
            const parsed = JSON.parse(raw) as {
              requestId?: string;
              error?: string;
              result?: unknown;
            };
            if (parsed.requestId !== requestId) {
              continue;
            }
            finish({ error: parsed.error, result: parsed.result ?? null });
            return;
          } catch (error) {
            finish({ error: error instanceof Error ? error.message : String(error) });
            return;
          }
        }
      });

      observer.observe(channel, { attributes: true, attributeFilter: [responseAttr] });
      channel.setAttribute(responseAttr, '');
      if (channel.getAttribute(readyAttr) !== '1') {
        channel.setAttribute(readyAttr, '1');
      }
      channel.setAttribute(requestAttr, requestPayload);

      const timeoutId = window.setTimeout(
        () => finish({ error: 'E2E channel timed out' }),
        timeoutMs,
      );
    },
    action,
    payload,
    {
      channelId: CHANNEL_ID,
      requestAttr: REQUEST_ATTR,
      responseAttr: RESPONSE_ATTR,
      readyAttr: READY_ATTR,
      timeoutMs: BRIDGE_TIMEOUT_MS,
    },
  );

  if (response?.error) {
    throw new Error(serializeError(response.error));
  }
  return (response?.result ?? null) as BridgeResultMap[T];
}

export async function resetAppState(): Promise<void> {
  await callBridge('resetAppState');
  await browser.refresh();
  await waitForAppReady();
}

export async function getAuthSnapshot(): Promise<AuthSnapshot> {
  return await callBridge('getAuthSnapshot');
}

export async function getOfflineSnapshot(): Promise<OfflineSnapshot> {
  return await callBridge('getOfflineSnapshot');
}

export async function getDirectMessageSnapshot(): Promise<DirectMessageSnapshot> {
  return await callBridge('getDirectMessageSnapshot');
}

export async function setAvatarFixture(fixture: AvatarFixture | null): Promise<void> {
  await callBridge('setProfileAvatarFixture', fixture ? { ...fixture } : null);
}

export async function seedDirectMessageConversation(
  params?: { content?: string; createdAt?: number },
): Promise<SeedDirectMessageConversationResult> {
  await waitForAppReady();
  try {
    return await callBridge('seedDirectMessageConversation', params ?? {});
  } catch (error) {
    const message = serializeError(error);
    const fallback = await browser.executeAsync<
      { result?: SeedDirectMessageConversationResult; error?: string },
      [{ content?: string; createdAt?: number }]
    >((payload, done) => {
      (async () => {
        try {
          // eslint-disable-next-line @typescript-eslint/no-unused-vars
          const toMessage = (err: unknown): string => {
            if (err instanceof Error) {
              return err.message;
            }
            if (err && typeof err === 'object') {
              try {
                return JSON.stringify(err);
              } catch {
                return String(err);
              }
            }
            return String(err);
          };
          if (typeof (window as Record<string, unknown>).__KUKURI_E2E_BOOTSTRAP__ === 'function') {
            (window as Record<string, unknown>).__KUKURI_E2E_BOOTSTRAP__?.();
          }
          const helper = (window as Record<string, unknown>).__KUKURI_E2E__;
          if (!helper || typeof helper.seedDirectMessageConversation !== 'function') {
            done({ error: 'E2E bridge helper is unavailable' });
            return;
          }
          const result = await helper.seedDirectMessageConversation(payload ?? {});
          done({ result: result ?? null });
        } catch (err) {
          done({ error: toMessage(err) });
        }
      })();
    }, params ?? {});

    if (fallback?.error) {
      throw new Error(`${message} (fallback failed: ${fallback.error})`);
    }
    return (fallback.result ??
      ((): SeedDirectMessageConversationResult => {
        throw new Error(`Bridge fallback returned no result: ${message}`);
      })()) as SeedDirectMessageConversationResult;
  }
}

export async function getTopicSnapshot(): Promise<TopicSnapshot> {
  return await callBridge('getTopicSnapshot');
}

export async function syncPendingTopicQueue(): Promise<SyncPendingTopicResult> {
  return await callBridge('syncPendingTopicQueue');
}

export async function seedTrendingFixture(
  fixture: TrendingFixture,
): Promise<SeedTrendingFixtureResult> {
  return await callBridge('seedTrendingFixture', fixture);
}

export async function seedUserSearchFixture(payload: {
  users: UserSearchFixtureUser[];
}): Promise<SeedUserSearchFixtureResult> {
  return await callBridge('seedUserSearchFixture', payload);
}

export async function primeUserSearchRateLimit(params?: {
  query?: string;
  limit?: number;
}): Promise<PrimeUserSearchRateLimitResult> {
  return await callBridge('primeUserSearchRateLimit', params ?? {});
}
