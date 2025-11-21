import { browser } from '@wdio/globals';
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
  | 'setProfileAvatarFixture'
  | 'consumeProfileAvatarFixture'
  | 'switchAccount';

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

export interface AvatarFixture {
  base64: string;
  format: string;
  fileName?: string;
}

type BridgeResultMap = {
  resetAppState: null;
  getAuthSnapshot: AuthSnapshot;
  getOfflineSnapshot: OfflineSnapshot;
  setProfileAvatarFixture: null;
  consumeProfileAvatarFixture: AvatarFixture | null;
  switchAccount: null;
};

declare global {
  interface Window {
    __KUKURI_E2E__?: E2EBridge;
    __KUKURI_E2E_BOOTSTRAP__?: () => Promise<void> | void;
  }
}

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
          const message = error instanceof Error ? error.message : String(error);
          return { error: message };
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
    throw new Error(response.error);
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

export async function setAvatarFixture(fixture: AvatarFixture | null): Promise<void> {
  await callBridge('setProfileAvatarFixture', fixture ? { ...fixture } : null);
}
