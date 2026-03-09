import { $, browser, expect } from '@wdio/globals';

import { getP2PStatus, resetAppState } from '../helpers/bridge';
import {
  completeProfileSetup,
  openSettings,
  waitForHome,
  waitForSettings,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';
import { waitForAppReady } from '../helpers/waitForAppReady';

const profile: ProfileInfo = {
  name: 'Reload Stability User',
  displayName: 'reload-stability-user',
  about: 'Community Node reload stability validation',
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

async function waitForAuthenticatedCommunityNode(normalizedBaseUrl: string): Promise<number> {
  let nodeIndex: number | null = null;
  await browser.waitUntil(
    async () => {
      nodeIndex = await findCommunityNodeIndex(normalizedBaseUrl);
      if (nodeIndex === null) {
        return false;
      }
      const tokenStatus = await $(`[data-testid="community-node-token-status-${nodeIndex}"]`);
      return (await tokenStatus.getAttribute('data-has-token')) === 'true';
    },
    {
      timeout: 30000,
      interval: 500,
      timeoutMsg: `Community Node ${normalizedBaseUrl} was not restored after reload`,
    },
  );

  if (nodeIndex === null) {
    throw new Error(`Community Node ${normalizedBaseUrl} was not found after reload`);
  }

  return nodeIndex;
}

describe('Community Node reload stability', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('keeps authenticated community node and p2p endpoint stable across repeated reloads', async function () {
    this.timeout(300000);

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

    const initialStatus = await getP2PStatus();
    expect(initialStatus.endpoint_id).toBeTruthy();
    expect(initialStatus.connection_status).not.toBe('error');
    const initialEndpointId = initialStatus.endpoint_id;

    for (let attempt = 0; attempt < 5; attempt += 1) {
      await browser.execute(() => {
        window.location.reload();
      });
      await waitForAppReady(60000);
      await waitForSettings();

      const nodeIndex = await waitForAuthenticatedCommunityNode(normalizedBaseUrl);
      const tokenStatus = await $(`[data-testid="community-node-token-status-${nodeIndex}"]`);
      expect(await tokenStatus.getAttribute('data-pubkey')).toBeTruthy();

      const status = await getP2PStatus();
      expect(status.endpoint_id).toBe(initialEndpointId);
      expect(status.connection_status).not.toBe('error');
    }
  });
});
