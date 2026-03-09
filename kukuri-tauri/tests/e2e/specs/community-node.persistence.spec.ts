import { $, browser, expect } from '@wdio/globals';

import { resetAppState } from '../helpers/bridge';
import {
  completeProfileSetup,
  openSettings,
  type ProfileInfo,
  waitForHome,
  waitForWelcome,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';
import { waitForAppReady } from '../helpers/waitForAppReady';

const profile: ProfileInfo = {
  name: 'Persistence User',
  displayName: 'persistence-user',
  about: 'Community Node persistence flow',
};

function normalizeUrl(value: string): string {
  return value.trim().replace(/\/+$/, '');
}

async function findCommunityNodeIndex(normalizedBaseUrl: string): Promise<number | null> {
  const nodes = await $$('[data-testid^="community-node-node-"]');
  for (let index = 0; index < nodes.length; index += 1) {
    const text = await nodes[index].getText();
    if (text.includes(normalizedBaseUrl)) {
      return index;
    }
  }
  return null;
}

describe('Community Node persistence', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('restores account and community node auth after app restart on Linux', async function () {
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl) {
      throw new Error('E2E_COMMUNITY_NODE_URL is not set');
    }
    const normalizedBaseUrl = normalizeUrl(baseUrl);

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await openSettings();
    await runCommunityNodeAuthFlow(baseUrl);

    await browser.reloadSession();
    await waitForAppReady(60000);
    await waitForHome();

    const pathname = await browser.execute(() => window.location.pathname);
    expect(pathname).not.toBe('/welcome');

    await openSettings();

    let nodeIndex: number | null = null;
    await browser.waitUntil(
      async () => {
        nodeIndex = await findCommunityNodeIndex(normalizedBaseUrl);
        return nodeIndex !== null;
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: `Community Node ${normalizedBaseUrl} was not restored after restart`,
      },
    );

    const tokenStatus = await $(`[data-testid="community-node-token-status-${nodeIndex}"]`);
    await tokenStatus.waitForDisplayed({ timeout: 15000 });
    await browser.waitUntil(
      async () => (await tokenStatus.getAttribute('data-has-token')) === 'true',
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: 'Community Node token was not restored after restart',
      },
    );
    expect(await tokenStatus.getAttribute('data-pubkey')).toBeTruthy();
  });
});
