import { browser } from '@wdio/globals';
import type { E2EBridge } from '@/testing/registerE2EBridge';
import { waitForAppReady } from './waitForAppReady';

export type BridgeAction = 'resetAppState' | 'getAuthSnapshot' | 'getOfflineSnapshot';

export interface AuthSnapshot {
  currentUser: {
    npub: string | null;
    displayName?: string | null;
    publicProfile?: boolean;
    showOnlineStatus?: boolean;
    picture?: string | null;
  } | null;
  accounts: Array<{ npub: string; display_name: string }>;
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
};

declare global {
  interface Window {
    __KUKURI_E2E__?: E2EBridge;
  }
}

export async function callBridge<T extends BridgeAction>(
  action: T,
): Promise<BridgeResultMap[T]> {
  const response = await browser.executeAsync<
    { error?: string; result?: BridgeResultMap[T] },
    [BridgeAction]
  >((name, done) => {
    const helper = window.__KUKURI_E2E__;
    if (!helper) {
      done({ error: 'E2E bridge is unavailable' });
      return;
    }
    const fn = helper[name];
    if (typeof fn !== 'function') {
      done({ error: `Unknown bridge action: ${name}` });
      return;
    }
    Promise.resolve(fn())
      .then((result) => done({ result: (result ?? null) as BridgeResultMap[T] }))
      .catch((error) => {
        const message = error instanceof Error ? error.message : String(error);
        done({ error: message });
      });
  }, action);

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
  await browser.execute(
    (payload) => {
      const helper = window.__KUKURI_E2E__;
      helper?.setProfileAvatarFixture?.(payload ?? null);
    },
    fixture ? { ...fixture } : null,
  );
}
