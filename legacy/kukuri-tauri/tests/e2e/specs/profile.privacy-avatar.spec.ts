import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  openSettings,
  type ProfileInfo,
} from '../helpers/appActions';
import { getAuthSnapshot, getOfflineSnapshot, resetAppState } from '../helpers/bridge';

const AVATAR_DATA_URL =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8Xw8AAtEB/QZWS+sAAAAASUVORK5CYII=';

async function safeClick(element: WebdriverIO.Element): Promise<void> {
  await element.scrollIntoView();
  await element.waitForClickable({ timeout: 10000 });
  try {
    await element.click();
  } catch {
    await browser.execute((target) => (target as HTMLElement | null)?.click(), element);
  }
}

async function getSwitchState(selector: string): Promise<string | null> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout: 10000 });
  return await element.getAttribute('data-state');
}

async function toggleSwitchToState(
  selector: string,
  expectedState: 'checked' | 'unchecked',
  timeoutMsg: string,
): Promise<void> {
  await browser.waitUntil(
    async () => {
      const currentState = await getSwitchState(selector);
      if (currentState === expectedState) {
        return true;
      }

      const element = await $(selector);
      try {
        await safeClick(element);
      } catch {
        // Fallback click is handled below.
      }

      await browser.pause(120);
      const afterNativeClick = await getSwitchState(selector);
      if (afterNativeClick === expectedState) {
        return true;
      }

      await browser.execute((cssSelector: string) => {
        const target = document.querySelector(cssSelector) as HTMLElement | null;
        target?.click();
      }, selector);
      await browser.pause(120);

      const afterFallbackClick = await getSwitchState(selector);
      return afterFallbackClick === expectedState;
    },
    {
      timeout: 12000,
      interval: 250,
      timeoutMsg,
    },
  );
}

describe('プロフィール/プライバシー/アバター同期', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('プライバシー設定とアバター同期がUIに反映されること', async function () {
    this.timeout(240000);

    await waitForWelcome();
    const initialProfile: ProfileInfo = {
      name: 'E2E プロフィール',
      displayName: 'privacy-e2e',
      about: '初期オンボーディングプロフィール',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(initialProfile);
    await waitForHome();

    await openSettings();

    await expect(await getSwitchState('#public-profile')).toBe('checked');
    await expect(await getSwitchState('#show-online')).toBe('unchecked');

    await toggleSwitchToState(
      '#public-profile',
      'unchecked',
      '公開プロフィールの状態が変わりませんでした',
    );
    await browser.waitUntil(
      async () => {
        const snapshot = await getAuthSnapshot();
        return snapshot.currentUser?.publicProfile === false;
      },
      { timeout: 15000, timeoutMsg: 'Authストアに非公開設定が反映されませんでした' },
    );

    await toggleSwitchToState(
      '#show-online',
      'checked',
      'オンライン表示の状態が変わりませんでした',
    );
    await browser.waitUntil(
      async () => {
        const snapshot = await getAuthSnapshot();
        return snapshot.currentUser?.showOnlineStatus === true;
      },
      { timeout: 15000, timeoutMsg: 'Authストアにオンライン表示が反映されませんでした' },
    );

    const offlineBefore = await getOfflineSnapshot();

    await $('[data-testid="open-profile-dialog"]').click();
    await $('[data-testid="profile-form"]').waitForDisplayed();

    const updatedProfile: ProfileInfo = {
      name: '同期テストユーザー',
      displayName: 'avatar-sync',
      about: 'プロフィールとアバターの同期を確認します',
    };

    await $('[data-testid="profile-name"]').setValue(updatedProfile.name);
    await $('[data-testid="profile-display-name"]').setValue(updatedProfile.displayName);
    await $('[data-testid="profile-about"]').setValue(updatedProfile.about);

    const pictureInput = await $('#picture');
    await pictureInput.setValue(AVATAR_DATA_URL);
    await browser.waitUntil(
      async () => {
        const value = await pictureInput.getValue();
        return value.startsWith('data:image/png');
      },
      { timeout: 5000, timeoutMsg: 'アバターURLが適用されませんでした' },
    );

    await $('[data-testid="profile-submit"]').click();
    await $('[data-testid="profile-form"]').waitForDisplayed({ reverse: true });

    await browser.waitUntil(
      async () => {
        const snapshot = await getAuthSnapshot();
        return (
          snapshot.currentUser?.displayName === updatedProfile.displayName &&
          snapshot.currentUser?.about === updatedProfile.about &&
          Boolean(snapshot.currentUser?.picture?.startsWith('data:image/png'))
        );
      },
      { timeout: 20000, timeoutMsg: 'プロフィール情報がAuthストアに反映されませんでした' },
    );

    const switcherText =
      (await $('[data-testid="account-switcher-trigger"]').getAttribute('aria-label')) ||
      (await $('[data-testid="account-switcher-trigger-text"]').getText());
    expect(switcherText.toLowerCase()).toContain(updatedProfile.displayName.toLowerCase());

    const avatarImg = await $('[data-testid="account-switcher-trigger"] img');
    const avatarSrc = await avatarImg.getAttribute('src');
    expect(avatarSrc).toContain('data:image/png');

    await browser.waitUntil(
      async () => {
        const snapshot = await getOfflineSnapshot();
        return (
          typeof snapshot.lastSyncedAt === 'number' &&
          snapshot.lastSyncedAt > (offlineBefore.lastSyncedAt ?? 0)
        );
      },
      {
        timeout: 20000,
        timeoutMsg: 'プロフィールアバター同期の完了がOfflineストアに反映されませんでした',
      },
    );

    const indicator = await $('[data-testid="offline-indicator-pill"]');
    if (await indicator.isExisting()) {
      await indicator.waitForDisplayed({ timeout: 10000 });
      const indicatorText = await indicator.getText();
      expect(indicatorText).toContain('最終同期');
    }
  });
});
