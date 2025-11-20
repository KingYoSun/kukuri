import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';

type BridgeAction = 'resetAppState' | 'getAuthSnapshot';

interface AuthSnapshot {
  currentUser: { npub: string | null; displayName?: string | null } | null;
  accounts: Array<{ npub: string; display_name: string }>;
}

type BridgeResultMap = {
  resetAppState: null;
  getAuthSnapshot: AuthSnapshot;
};

declare global {
  interface Window {
    __KUKURI_E2E__?: {
      resetAppState: () => Promise<void>;
      getAuthSnapshot: () => AuthSnapshot;
    };
  }
}

interface ProfileInfo {
  name: string;
  displayName: string;
  about: string;
}

async function callBridge<T extends BridgeAction>(action: T): Promise<BridgeResultMap[T]> {
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

async function resetAppState(): Promise<void> {
  await callBridge('resetAppState');
  await browser.refresh();
  await waitForAppReady();
}

async function waitForWelcome(): Promise<void> {
  const welcome = await $('[data-testid="welcome-screen"]');
  await welcome.waitForDisplayed();
}

async function completeProfileSetup(profile: ProfileInfo): Promise<void> {
  const form = await $('[data-testid="profile-form"]');
  await form.waitForDisplayed();

  await $('[data-testid="profile-name"]').setValue(profile.name);
  await $('[data-testid="profile-display-name"]').setValue(profile.displayName);
  await $('[data-testid="profile-about"]').setValue(profile.about);
  await $('[data-testid="profile-submit"]').click();
}

async function waitForHome(): Promise<void> {
  const home = await $('[data-testid="home-page"]');
  await home.waitForDisplayed();
}

async function openSettings(): Promise<void> {
  await $('[data-testid="open-settings-button"]').click();
  await $('[data-testid="settings-page"]').waitForDisplayed();
}

async function openAccountMenu(): Promise<void> {
  await $('[data-testid="account-switcher-trigger"]').click();
  await $('[data-testid="account-menu-go-login"]').waitForDisplayed();
}

describe('オンボーディングとキー管理', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('新規作成/設定同期/鍵エクスポート/再インポート/アカウント切替が行えること', async function () {
    this.timeout(180000);

    await waitForWelcome();

    const profileA: ProfileInfo = {
      name: 'E2E アカウントA',
      displayName: 'onboard-a',
      about: 'アカウントAの自己紹介',
    };
    const profileB: ProfileInfo = {
      name: 'E2E アカウントB',
      displayName: 'onboard-b',
      about: 'アカウントBの自己紹介',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profileA);
    await waitForHome();

    const snapshotAfterA = await callBridge('getAuthSnapshot');
    const npubA = snapshotAfterA.currentUser?.npub;
    expect(npubA).toBeTruthy();

    const switcherLabelA = await $('[data-testid="account-switcher-trigger"]').getText();
    expect(switcherLabelA.toLowerCase()).toContain(profileA.displayName.toLowerCase());

    await openSettings();
    await $('[data-testid="open-profile-dialog"]').click();
    await $('[data-testid="profile-form"]').waitForDisplayed();
    await expect(await $('[data-testid="profile-name"]').getValue()).toBe(profileA.name);
    await expect(await $('[data-testid="profile-display-name"]').getValue()).toBe(
      profileA.displayName,
    );
    await expect(await $('[data-testid="profile-about"]').getValue()).toBe(profileA.about);
    await $('[data-testid="profile-cancel"]').click();

    await $('[data-testid="open-key-dialog"]').click();
    await $('[data-testid="key-management-dialog"]').waitForDisplayed();
    await $('[data-testid="key-export-button"]').click();
    await browser.waitUntil(async () => {
      const value = await $('[data-testid="key-exported-value"]').getValue();
      return Boolean(value);
    });
    const exportedKey = await $('[data-testid="key-exported-value"]').getValue();
    expect(exportedKey).toMatch(/^nsec1/);
    await browser.keys('Escape');

    await openAccountMenu();
    await $('[data-testid="account-menu-logout"]').click();
    await waitForWelcome();

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profileB);
    await waitForHome();

    const snapshotAfterB = await callBridge('getAuthSnapshot');
    const npubB = snapshotAfterB.currentUser?.npub;
    expect(npubB).toBeTruthy();

    await openSettings();
    await $('[data-testid="open-key-dialog"]').click();
    await $('[data-testid="key-management-dialog"]').waitForDisplayed();
    await $('[data-testid="key-tab-import"]').click();
    await $('[data-testid="key-import-input"]').setValue(exportedKey);
    await $('[data-testid="key-import-button"]').click();
    await browser.waitUntil(async () => {
      const snapshot = await callBridge('getAuthSnapshot');
      return snapshot.currentUser?.npub === npubA;
    });
    await browser.keys('Escape');

    await openAccountMenu();
    let switchOptions = await $$('[data-testid="account-switch-option"]');
    expect(switchOptions.length).toBeGreaterThan(0);
    await switchOptions[0]!.click();
    await browser.waitUntil(async () => {
      const snapshot = await callBridge('getAuthSnapshot');
      return snapshot.currentUser?.npub === npubB;
    });

    await openAccountMenu();
    switchOptions = await $$('[data-testid="account-switch-option"]');
    expect(switchOptions.length).toBeGreaterThan(0);
    await switchOptions[0]!.click();
    await browser.waitUntil(async () => {
      const snapshot = await callBridge('getAuthSnapshot');
      return snapshot.currentUser?.npub === npubA;
    });
  });
});
