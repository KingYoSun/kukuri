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

const profile: ProfileInfo = {
  name: 'E2E Community',
  displayName: 'community-node',
  about: 'Community Node settings flow',
};

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

    const baseInput = await $('[data-testid="community-node-base-url"]');
    await baseInput.waitForDisplayed({ timeout: 20000 });
    await baseInput.setValue(baseUrl);
    await $('[data-testid="community-node-save-config"]').click();

    const authButton = await $('[data-testid="community-node-authenticate"]');
    await browser.waitUntil(async () => await authButton.isEnabled(), {
      timeout: 15000,
      interval: 300,
      timeoutMsg: 'Community node auth button did not become enabled',
    });
    await authButton.click();

    const status = await $('[data-testid="community-node-token-status"]');
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
      async () => (await status.getAttribute('data-has-token')) === 'false',
      {
        timeout: 15000,
        interval: 300,
        timeoutMsg: 'Community node config was not cleared',
      },
    );
  });
});
