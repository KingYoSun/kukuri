import { $, $$, browser, expect } from '@wdio/globals';
import { bech32 } from '@scure/base';
import { waitForAppReady } from '../helpers/waitForAppReady';
import { ensureTestTopic, resetAppState } from '../helpers/bridge';
import { openSettings, waitForHome, waitForWelcome } from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

type SeedPostSummary = {
  event_id: string;
  topic_id: string;
  content: string;
  created_at: number;
  author_pubkey?: string;
};

type SeedSummary = {
  post?: SeedPostSummary;
};

const SEED_SUBSCRIBER_SECRET =
  '000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f';

const parseSeedSummary = (): SeedSummary | null => {
  const raw = process.env.E2E_COMMUNITY_NODE_SEED_JSON;
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw) as SeedSummary;
  } catch {
    return null;
  }
};

const hexToBytes = (hex: string): Uint8Array => {
  const cleaned = hex.trim().replace(/^0x/i, '');
  if (cleaned.length % 2 !== 0) {
    throw new Error('Invalid hex length for nsec conversion');
  }
  const bytes = new Uint8Array(cleaned.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(cleaned.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
};

const deriveNsec = (hex: string) =>
  bech32.encode('nsec', bech32.toWords(hexToBytes(hex)), 1023);

describe('Community Node search/index', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('connects search UI to community node index with suggest/paging/0 results', async function () {
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const scenario = process.env.SCENARIO;
    const seed = parseSeedSummary();
    const topicId = seed?.post?.topic_id ?? 'kukuri:e2e-alpha';

    if (!baseUrl || scenario !== 'community-node-e2e' || !topicId) {
      this.skip();
      return;
    }

    await waitForWelcome();
    await $('[data-testid="welcome-login"]').click();

    const nsecInput = await $('#nsec');
    await nsecInput.waitForDisplayed({ timeout: 20000 });
    await nsecInput.setValue(deriveNsec(SEED_SUBSCRIBER_SECRET));
    await $('button[type="submit"]').click();

    await waitForHome();

    await ensureTestTopic({ name: 'community-node-search', topicId });
    const topicButton = await $(`[data-testid="topic-${topicId}"]`);
    await topicButton.waitForDisplayed({ timeout: 20000 });
    await topicButton.scrollIntoView();
    await topicButton.click();

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
    await runCommunityNodeAuthFlow(baseUrl);

    const acceptConsents = await $('[data-testid="community-node-accept-consents"]');
    await acceptConsents.waitForClickable({ timeout: 15000 });
    await acceptConsents.click();

    const consents = await $('[data-testid="community-node-consents"]');
    await browser.waitUntil(
      async () => {
        const text = await consents.getText();
        return text.includes('accepted') || text.includes('consent') || text.includes('policies');
      },
      { timeout: 20000, interval: 500, timeoutMsg: 'Community node consents did not update' },
    );

    const searchToggle = await $('#community-node-search');
    await searchToggle.waitForDisplayed({ timeout: 10000 });
    const isChecked = (await searchToggle.getAttribute('aria-checked')) === 'true';
    if (!isChecked) {
      await searchToggle.click();
    }

    const topicButtonAfterSettings = await $(`[data-testid="topic-${topicId}"]`);
    await topicButtonAfterSettings.waitForDisplayed({ timeout: 20000 });
    await topicButtonAfterSettings.scrollIntoView();
    await topicButtonAfterSettings.click();
    await waitForHome();

    await browser.execute(() => {
      try {
        window.history.pushState({}, '', '/search');
      } catch {
        window.location.replace('/search');
      }
    });

    await $('[data-testid="search-page"]').waitForDisplayed({ timeout: 20000 });
    const searchInput = await $('[data-testid="search-input"]');
    await searchInput.waitForDisplayed({ timeout: 10000 });
    await searchInput.clearValue();
    await searchInput.setValue('Alpha');

    await browser.waitUntil(
      async () => (await $$('[data-testid="community-node-search-result"]')).length > 0,
      { timeout: 30000, interval: 500, timeoutMsg: 'Community node search results did not appear' },
    );
    const initialResults = await $$('[data-testid="community-node-search-result"]');
    expect(initialResults.length).toBeGreaterThan(0);

    const loadMore = await $('[data-testid="community-node-search-load-more"]');
    await loadMore.waitForDisplayed({ timeout: 20000 });
    await loadMore.click();

    await browser.waitUntil(
      async () =>
        (await $$('[data-testid="community-node-search-result"]')).length >
        initialResults.length,
      { timeout: 30000, interval: 500, timeoutMsg: 'Community node search did not paginate' },
    );

    await searchInput.clearValue();
    await searchInput.setValue('zzzz-no-hit');
    await $('[data-testid="community-node-search-empty"]').waitForDisplayed({ timeout: 20000 });
  });
});
