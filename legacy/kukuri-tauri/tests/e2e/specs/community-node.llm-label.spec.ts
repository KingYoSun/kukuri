import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import { ensureTestTopic, resetAppState, seedCommunityNodePost } from '../helpers/bridge';
import {
  completeProfileSetup,
  openSettings,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

type SeedPostSummary = {
  event_id: string;
  author_pubkey: string;
  topic_id: string;
  content: string;
  created_at: number;
};

type SeedSummary = {
  llm_label?: string;
  llm_label_confidence?: number;
  llm_post?: SeedPostSummary;
};

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

describe('Community Node LLM label badge', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('shows LLM generated label for seeded post', async function () {
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const scenario = process.env.SCENARIO;
    const seed = parseSeedSummary();
    if (!baseUrl || scenario !== 'community-node-e2e' || !seed?.llm_post || !seed.llm_label) {
      this.skip();
      return;
    }

    const post = seed.llm_post;
    if (!post.event_id || !post.author_pubkey) {
      throw new Error('Seed summary is missing llm post identifiers');
    }

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Community LLM',
      displayName: 'community-node-llm',
      about: 'Community node llm label flow',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await openSettings();

    const baseInput = await $('[data-testid="community-node-base-url"]');
    await baseInput.waitForDisplayed({ timeout: 20000 });
    await baseInput.setValue(baseUrl);
    await $('[data-testid="community-node-save-config"]').click();

    const authButton = await $('[data-testid="community-node-authenticate-0"]');
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

    await browser.execute(() => {
      try {
        window.history.pushState({}, '', '/');
      } catch {
        window.location.replace('/');
      }
    });
    await waitForHome();

    await ensureTestTopic({ name: 'community-node-llm', topicId: post.topic_id });
    await seedCommunityNodePost({
      id: post.event_id,
      content: post.content || 'Community node llm label seed post',
      authorPubkey: post.author_pubkey,
      topicId: post.topic_id,
      createdAt: post.created_at,
      authorName: 'Community Node LLM Seed',
      authorDisplayName: 'Community Node LLM Seed',
    });

    const baseTestId = `post-${post.event_id}`;
    const card = await $(`[data-testid="${baseTestId}"]`);
    await card.waitForDisplayed({ timeout: 30000 });

    const labelBadge = await card.$(`[data-testid="${baseTestId}-label-0"]`);
    await browser.waitUntil(async () => await labelBadge.isExisting(), {
      timeout: 30000,
      interval: 500,
      timeoutMsg: 'LLM label badge did not render',
    });
    await browser.waitUntil(
      async () => {
        const value = await labelBadge.getAttribute('data-label');
        return Boolean(value && value.trim().length > 0);
      },
      { timeout: 30000, interval: 500, timeoutMsg: 'LLM label text did not resolve' },
    );

    const labelValue = (await labelBadge.getAttribute('data-label')) ?? '';
    expect(labelValue).toContain(seed.llm_label);
    if (typeof seed.llm_label_confidence === 'number') {
      const confidenceText = seed.llm_label_confidence.toFixed(2);
      expect(labelValue).toContain(confidenceText);
    }
  });
});
