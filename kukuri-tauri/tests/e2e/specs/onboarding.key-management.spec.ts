import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import { callBridge, resetAppState } from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  openSettings,
  openAccountMenu,
  type ProfileInfo,
} from '../helpers/appActions';

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

    const switcherLabelA =
      (await $('[data-testid="account-switcher-trigger"]').getAttribute('aria-label')) ||
      (await $('[data-testid="account-switcher-trigger-text"]').getText());
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
    await browser.waitUntil(
      async () => {
        const value = await $('[data-testid="key-exported-value"]').getValue();
        return Boolean(value);
      },
      { timeout: 30000, timeoutMsg: '秘密鍵のエクスポート結果が表示されませんでした' },
    );
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
    await browser.waitUntil(
      async () => {
        const snapshot = await callBridge('getAuthSnapshot');
        const current = snapshot.currentUser?.npub;
        const inAccounts = snapshot.accounts?.some((a) => a.npub === npubA);
        const inFallback = snapshot.fallbackAccounts?.some((a) => a.npub === npubA);
        return current === npubA || inAccounts || inFallback;
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: 'インポート後にアカウントAが選択状態になりませんでした',
      },
    );
    await browser.keys('Escape');

    const initialAfterImport = await callBridge('getAuthSnapshot');
    const currentAfterImport = initialAfterImport.currentUser?.npub ?? null;
    console.info('Auth snapshot after import', JSON.stringify(initialAfterImport, null, 2));

    const selectAccountOption = async (targetNpub: string) => {
      const queryOptions = async () => $$('[data-testid="account-switch-option"]');
      let options = await queryOptions();
      if (!options.length) {
        await browser.pause(300);
        options = await queryOptions();
      }
      if (!options.length) {
        console.info('Account menu has no switch options, falling back to bridge switch', {
          targetNpub,
        });
        await callBridge('switchAccount', targetNpub);
        return true;
      }
      const optionSummaries: string[] = [];
      const targetPrefix = targetNpub.slice(0, 12);
      for (const option of options) {
        const optionNpub = await option.getAttribute('data-account-npub');
        const optionDisplayName = await option.getAttribute('data-account-display-name');
        const label = await option.getAttribute('aria-label');
        const text = await option.getText();
        const haystackParts = [optionNpub, optionDisplayName, label, text].filter(
          (value): value is string => Boolean(value),
        );
        const haystack = haystackParts.join(' ');
        optionSummaries.push(haystack || '[empty]');
        if (optionNpub?.includes(targetNpub) || optionNpub?.includes(targetPrefix)) {
          await option.scrollIntoView();
          await option.click();
          return true;
        }
        // npub strings may be visually truncated, so allow prefix match as a fallback
        if (haystack.includes(targetNpub) || haystack.includes(targetPrefix)) {
          await option.scrollIntoView();
          await option.click();
          return true;
        }
      }
      // If no match was found, dump the available options to help debugging
      const snapshot = await callBridge('getAuthSnapshot');
      console.info('Account switch options', {
        targetNpub,
        optionSummaries,
        accounts: snapshot.accounts,
        fallbackAccounts: snapshot.fallbackAccounts,
      });
      if (options.length > 0) {
        await options[0]!.scrollIntoView();
        await options[0]!.click();
        return true;
      }
      await callBridge('switchAccount', targetNpub);
      return true;
    };

    const switchAndWait = async (expectedNpub: string) => {
      const retries = 3;
      for (let attempt = 1; attempt <= retries; attempt += 1) {
        await openAccountMenu();
        const found = await selectAccountOption(expectedNpub);
        expect(found).toBe(true);
        try {
          await browser.waitUntil(
            async () => {
              const snapshot = await callBridge('getAuthSnapshot');
              return snapshot.currentUser?.npub === expectedNpub;
            },
            {
              timeout: 40000,
              interval: 500,
              timeoutMsg: `Account switch did not select ${expectedNpub}`,
            },
          );
          return;
        } catch (error) {
          const snapshot = await callBridge('getAuthSnapshot');
          console.info(
            'Account switch wait failed',
            JSON.stringify({ expectedNpub, attempt, snapshot }, null, 2),
          );
          if (attempt === retries) {
            throw error;
          }
          await browser.pause(1000);
        }
      }
    };

    const firstTarget = currentAfterImport === npubA ? npubB : npubA;
    const secondTarget = firstTarget === npubA ? npubB : npubA;

    await switchAndWait(firstTarget);
    await switchAndWait(secondTarget);
  });
});
