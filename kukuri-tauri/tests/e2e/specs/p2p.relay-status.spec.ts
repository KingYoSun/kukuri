import { $, $$, browser, expect } from '@wdio/globals';

import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import {
  resetAppState,
  getBootstrapSnapshot,
  clearBootstrapNodes,
  type BootstrapSnapshot,
} from '../helpers/bridge';
import { waitForAppReady } from '../helpers/waitForAppReady';

const profile: ProfileInfo = {
  name: 'E2E Relay',
  displayName: 'relay-status',
  about: 'RelayStatus / CLI bootstrap E2E',
};

describe('P2P / RelayStatus / CLIブートストラップ', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
    try {
      await clearBootstrapNodes();
    } catch (error) {
      console.info(
        'clearBootstrapNodes failed',
        error instanceof Error ? error.message : String(error),
      );
    }
  });

  it('CLIブートストラップを検知し適用できる', async function () {
    this.timeout(180000);

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    const initialSnapshot: BootstrapSnapshot = await getBootstrapSnapshot();
    expect(initialSnapshot.cliNodes.length).toBeGreaterThan(0);
    expect(initialSnapshot.envLocked).toBe(false);

    const openNetworkStatusButton = await $('[data-testid="open-network-status-button"]');
    await openNetworkStatusButton.waitForClickable({ timeout: 30000 });
    await openNetworkStatusButton.click();

    const networkStatusModal = await $('[data-testid="network-status-modal"]');
    await networkStatusModal.waitForDisplayed({ timeout: 30000 });

    const relayCard = await $('[data-testid="relay-status-card"]');
    await relayCard.waitForDisplayed({ timeout: 30000 });
    await relayCard.scrollIntoView();

    await browser.waitUntil(
      async () => {
        const cliInfo = await $('[data-testid="relay-cli-info"]');
        const countAttr = await cliInfo.getAttribute('data-cli-count');
        const parsedCount = countAttr ? Number(countAttr) : NaN;
        return Number.isFinite(parsedCount) && parsedCount === initialSnapshot.cliNodes.length;
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: 'CLIブートストラップ情報が表示されませんでした',
      },
    );

    const applyButton = await $('[data-testid="relay-apply-cli-button"]');
    await browser.waitUntil(async () => await applyButton.isEnabled(), {
      timeout: 20000,
      interval: 300,
      timeoutMsg: 'CLI適用ボタンが有効化されませんでした',
    });
    await applyButton.click();

    let finalSnapshot: BootstrapSnapshot = initialSnapshot;
    await browser.waitUntil(
      async () => {
        finalSnapshot = await getBootstrapSnapshot();
        const normalizedInitial = [...initialSnapshot.cliNodes].sort().join('|');
        const normalizedEffective = [...finalSnapshot.effectiveNodes].sort().join('|');
        return (
          finalSnapshot.source === 'user' &&
          finalSnapshot.effectiveNodes.length === finalSnapshot.cliNodes.length &&
          normalizedEffective === normalizedInitial
        );
      },
      {
        timeout: 40000,
        interval: 1000,
        timeoutMsg: 'CLIブートストラップ適用結果が反映されませんでした',
      },
    );

    const effectiveCount = await $('[data-testid="relay-effective-count"]');
    const effectiveCountAttr = await effectiveCount.getAttribute('data-count');
    expect(Number(effectiveCountAttr)).toBe(finalSnapshot.effectiveNodes.length);

    await browser.waitUntil(
      async () => {
        const relays = await $$('[data-testid="relay-status-item"]');
        return relays.length === finalSnapshot.effectiveNodes.length;
      },
      { timeout: 30000, interval: 500, timeoutMsg: 'RelayStatusの件数が更新されませんでした' },
    );

    const runbookLink = await $('[data-testid="relay-runbook-link"]');
    expect(await runbookLink.getAttribute('href')).toContain('p2p_mainline_runbook');
  });
});
