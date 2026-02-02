import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  ensureTestTopic,
  resetAppState,
  seedCommunityNodePost,
} from '../helpers/bridge';
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
  label?: string;
  trust_report_score?: number;
  trust_density_score?: number;
  post?: SeedPostSummary;
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

describe('Community Node labels/trust badges', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('shows label and trust badges for seeded post', async function () {
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const scenario = process.env.SCENARIO;
    const seed = parseSeedSummary();
    if (!baseUrl || scenario !== 'community-node-e2e' || !seed?.post) {
      this.skip();
      return;
    }

    const post = seed.post;
    if (!post.event_id || !post.author_pubkey) {
      throw new Error('Seed summary is missing post identifiers');
    }

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Community Seed',
      displayName: 'community-node-seed',
      about: 'Community node label/trust E2E flow',
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

    const topicId = post.topic_id || 'kukuri:e2e-alpha';
    await ensureTestTopic({ name: 'community-node-seed', topicId });

    await seedCommunityNodePost({
      id: post.event_id,
      content: post.content || 'Community node seed post',
      authorPubkey: post.author_pubkey,
      topicId,
      createdAt: post.created_at,
      authorName: 'Community Node Seed',
      authorDisplayName: 'Community Node Seed',
    });

    const baseTestId = `post-${post.event_id}`;
    const card = await $(`[data-testid="${baseTestId}"]`);
    await card.waitForDisplayed({ timeout: 30000 });

    const labelBadge = await card.$(`[data-testid="${baseTestId}-label-0"]`);
    await browser.waitUntil(async () => await labelBadge.isExisting(), {
      timeout: 20000,
      interval: 500,
      timeoutMsg: 'Label badge did not render',
    });
    await browser.waitUntil(async () => {
      const value = await labelBadge.getAttribute('data-label');
      return Boolean(value && value.trim().length > 0);
    }, { timeout: 20000, interval: 500, timeoutMsg: 'Label badge text did not resolve' });
    const labelValue = (await labelBadge.getAttribute('data-label')) ?? '';
    if (seed.label) {
      expect(labelValue).toContain(seed.label);
    } else {
      expect(labelValue.length).toBeGreaterThan(0);
    }

    const reportBadge = await card.$(`[data-testid="${baseTestId}-trust-report"]`);
    await reportBadge.waitForDisplayed({ timeout: 20000 });
    if (typeof seed.trust_report_score === 'number') {
      const scoreValue = Number.parseFloat(
        (await reportBadge.getAttribute('data-score')) ?? 'NaN',
      );
      expect(scoreValue).toBeCloseTo(seed.trust_report_score, 2);
    }

    const densityBadge = await card.$(`[data-testid="${baseTestId}-trust-density"]`);
    await densityBadge.waitForDisplayed({ timeout: 20000 });
    if (typeof seed.trust_density_score === 'number') {
      const scoreValue = Number.parseFloat(
        (await densityBadge.getAttribute('data-score')) ?? 'NaN',
      );
      expect(scoreValue).toBeCloseTo(seed.trust_density_score, 2);
    }
  });
});
