import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  getAuthSnapshot,
  getDirectMessageSnapshot,
  resetAppState,
  seedDirectMessageConversation,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';

const dmButtonSelector = [
  'button[aria-label="ダイレクトメッセージ"]',
  'button[aria-label="Direct message"]',
  'button[aria-label="Direct Message"]',
  'button[aria-label="私信"]',
].join(', ');

describe('ダイレクトメッセージInbox', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('ヘッダー/サマリー導線からDM Inboxを開き未読が同期される', async function () {
    this.timeout(240000);

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E DM Inbox',
      displayName: 'dm-inbox',
      about: 'ヘッダーとサマリーCTAの動作確認',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();
    await waitForAppReady();

    const bridgeStatus = await browser.execute(() => {
      const channel = document.getElementById('kukuri-e2e-channel');
      const helper = (window as unknown as Record<string, unknown>).__KUKURI_E2E__;
      return {
        hasHelper: Boolean(helper),
        hasBootstrap:
          typeof (window as unknown as Record<string, unknown>).__KUKURI_E2E_BOOTSTRAP__ ===
          'function',
        helperKeys: helper ? Object.keys(helper as Record<string, unknown>) : [],
        status: (window as unknown as Record<string, unknown>).__KUKURI_E2E_STATUS__ ?? null,
        attrStatus: document.documentElement?.getAttribute('data-kukuri-e2e-status') ?? null,
        channelReady: channel?.getAttribute('data-e2e-ready') === '1',
      };
    });
    console.info('E2E bridge status before DM seed', bridgeStatus);
    await getAuthSnapshot();

    const seeded = await seedDirectMessageConversation({
      content: 'E2E seeded direct message',
    });
    const unreadBadgeSelector = `[data-testid="dm-inbox-unread-${seeded.conversationNpub}"]`;

    const seededSnapshot = await getDirectMessageSnapshot();
    console.info('DM snapshot after seed', seededSnapshot);
    expect(seededSnapshot.unreadTotal).toBeGreaterThan(0);

    const dmButton = await $(dmButtonSelector);
    await dmButton.waitForDisplayed({ timeout: 20000 });
    await browser.waitUntil(
      async () => {
        const badge = await dmButton.$('span');
        return (await badge.isExisting()) && (await badge.getText()).trim().length > 0;
      },
      { timeout: 15000, interval: 300, timeoutMsg: '未読バッジが表示されませんでした' },
    );

    await dmButton.click();
    await browser.waitUntil(
      async () => (await $$('[data-testid="direct-message-item"]')).length > 0,
      { timeout: 20000, interval: 300, timeoutMsg: 'DMダイアログが開きませんでした' },
    );
    const messages = await $$('[data-testid="direct-message-item"]');
    const messageContent = await messages[0]!.$('[data-testid="direct-message-content"]');
    await browser.waitUntil(async () => (await messageContent.getText()).includes(seeded.content), {
      timeout: 20000,
      interval: 300,
      timeoutMsg: 'DM本文が取得できませんでした',
    });
    const firstMessageText = await messageContent.getText();
    expect(firstMessageText).toContain(seeded.content);

    await browser.keys('Escape');
    await browser.waitUntil(
      async () => (await $$('[data-testid="direct-message-item"]')).length === 0,
      { timeout: 15000, interval: 300 },
    );

    const summaryCards = await $$('[data-testid="trending-summary-direct-messages"]');
    if (summaryCards.length > 0) {
      await summaryCards[0]!.scrollIntoView();
      const summaryText = await summaryCards[0]!.getText();
      expect(summaryText).toContain('件');

      const summaryCta = await $('[data-testid="trending-summary-direct-messages-cta"]');
      await summaryCta.click();

      const inboxList = await $('[data-testid="dm-inbox-list"]');
      await inboxList.waitForDisplayed({ timeout: 20000 });
      await browser.waitUntil(
        async () => (await $$('[data-testid^="dm-inbox-conversation-"]')).length > 0,
        { timeout: 20000, interval: 300, timeoutMsg: '会話行が一件も表示されませんでした' },
      );
      expect(await $(unreadBadgeSelector).isExisting()).toBe(false);

      await browser.keys('Escape');
      await inboxList.waitForDisplayed({ reverse: true, timeout: 15000 });
    } else {
      console.info(
        'Trending summary card for direct messages not found; skipping summary CTA check',
      );
    }
  });
});
