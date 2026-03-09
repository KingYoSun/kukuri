import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import { resetAppState } from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  openSettings,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';
import {
  expectNoToastMatching,
  waitForToastsToClear,
} from '../helpers/toasts';

const profile: ProfileInfo = {
  name: 'E2E Community',
  displayName: 'community-node',
  about: 'Community Node settings flow',
};

async function getSwitchState(selector: string): Promise<string | null> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout: 10000 });
  return await element.getAttribute('data-state');
}

describe('Community Node settings', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('saves config and authenticates against the community node endpoint', async function () {
    this.timeout(180000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl) {
      throw new Error('E2E_COMMUNITY_NODE_URL is not set');
    }

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await openSettings();
    await waitForToastsToClear().catch(() => {});

    const baseInput = await $('[data-testid="community-node-base-url"]');
    await baseInput.waitForDisplayed({ timeout: 20000 });
    await baseInput.setValue(baseUrl);
    await $('[data-testid="community-node-save-config"]').click();
    await expectNoToastMatching({
      patterns: ['Community Node の追加に失敗しました'],
      description: 'community node config save should not show failure toast',
    });

    const authButton = await $('[data-testid="community-node-authenticate-0"]');
    await browser.waitUntil(async () => await authButton.isEnabled(), {
      timeout: 15000,
      interval: 300,
      timeoutMsg: 'Community node auth button did not become enabled',
    });
    await runCommunityNodeAuthFlow(baseUrl);

    const status = await $('[data-testid="community-node-token-status-0"]');
    await browser.waitUntil(
      async () => (await status.getAttribute('data-has-token')) === 'true',
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: 'Community node token was not set',
      },
    );
    const pubkey = await status.getAttribute('data-pubkey');
    expect(pubkey).toBeTruthy();
    await expectNoToastMatching({
      patterns: ['Community Node 認証に失敗しました', 'Trust Provider の取得に失敗しました'],
      durationMs: 5000,
      description: 'community node authenticate should not show false failure toast',
    });

    const searchSwitchSelector = '#community-node-role-search-0';
    const beforeSearchState = await getSwitchState(searchSwitchSelector);
    await $(searchSwitchSelector).click();
    await browser.waitUntil(
      async () => {
        const nextState = await getSwitchState(searchSwitchSelector);
        return nextState !== beforeSearchState;
      },
      {
        timeout: 20000,
        interval: 300,
        timeoutMsg: 'Community node role toggle did not update',
      },
    );
    await expectNoToastMatching({
      patterns: ['Community Node ロール更新に失敗しました'],
      description: 'community node role toggle should not show false failure toast',
    });

    const consents = await $('[data-testid="community-node-consents"]');
    await browser.waitUntil(
      async () => {
        const text = await consents.getText();
        return text.includes('policies') || text.includes('consents') || text.includes('accepted');
      },
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: 'Community node consents did not load',
      },
    );

    await $('[data-testid="community-node-clear-config"]').click();
    await browser.waitUntil(
      async () => (await $$('[data-testid^="community-node-node-"]')).length === 0,
      {
        timeout: 15000,
        interval: 300,
        timeoutMsg: 'Community node config was not cleared',
      },
    );
  });
});
