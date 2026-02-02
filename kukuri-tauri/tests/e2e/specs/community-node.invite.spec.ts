import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import { resetAppState, ensureTestTopic } from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  openSettings,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

type InviteEvent = {
  tags?: Array<string[]>;
};

const INVITE_JSON = process.env.E2E_COMMUNITY_NODE_INVITE_JSON;
const TOPIC_NAME = process.env.E2E_COMMUNITY_NODE_TOPIC_NAME ?? 'e2e-community-node-invite';

const getTagValue = (event: InviteEvent | null, tagName: string): string | null => {
  if (!event?.tags) {
    return null;
  }
  for (const tag of event.tags) {
    if (Array.isArray(tag) && tag[0] === tagName && typeof tag[1] === 'string') {
      return tag[1];
    }
  }
  return null;
};

const selectTopicByName = async (topicName: string) => {
  const selector = await $('[data-testid="topic-selector"]');
  await selector.waitForDisplayed({ timeout: 20000 });
  await selector.click();
  const searchInput = await $('input[data-slot="command-input"]');
  await searchInput.waitForDisplayed({ timeout: 10000 });
  await searchInput.setValue(topicName);
  const item = await $(`//div[@data-slot="command-item" and contains(., "${topicName}")]`);
  await item.waitForDisplayed({ timeout: 15000 });
  await item.click();
};

const selectInviteScope = async () => {
  const scopeTrigger = await $('[data-testid="scope-selector"]');
  await scopeTrigger.waitForDisplayed({ timeout: 15000 });
  await scopeTrigger.click();
  const inviteItem = await $(`//div[@data-slot="select-item" and contains(., "招待")]`);
  await inviteItem.waitForDisplayed({ timeout: 10000 });
  await inviteItem.click();
};

describe('Community Node invite flow', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('redeems invite, syncs keys, and posts encrypted content', async function () {
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl || !INVITE_JSON) {
      this.skip();
      return;
    }

    const inviteEvent = JSON.parse(INVITE_JSON) as InviteEvent;
    const topicId = getTagValue(inviteEvent, 't');
    const scope = getTagValue(inviteEvent, 'scope') ?? 'invite';
    if (!topicId) {
      throw new Error('Invite topic id is missing');
    }
    expect(scope).toBe('invite');

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Invite User',
      displayName: 'community-node-invite',
      about: 'Community Node invite/key sync/encrypted post flow',
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

    const inviteInput = await $('[data-testid="community-node-invite-json"]');
    await inviteInput.waitForDisplayed({ timeout: 15000 });
    await inviteInput.setValue(INVITE_JSON);
    await $('[data-testid="community-node-redeem-invite"]').click();

    const settingsPage = await $('[data-testid="settings-page"]');
    await browser.waitUntil(async () => (await settingsPage.getText()).includes(topicId ?? ''), {
      timeout: 30000,
      interval: 500,
      timeoutMsg: 'Key envelope was not stored after invite redeem',
    });

    const topicInput = await $('#key-sync-topic-id');
    await topicInput.waitForDisplayed({ timeout: 15000 });
    await topicInput.setValue(topicId ?? '');
    await $('[data-testid="community-node-sync-keys"]').click();

    await browser.waitUntil(async () => (await settingsPage.getText()).includes(topicId ?? ''), {
      timeout: 30000,
      interval: 500,
      timeoutMsg: 'Key sync did not reflect invite topic',
    });

    const topic = await ensureTestTopic({ name: TOPIC_NAME, topicId });
    expect(topic.id).toBe(topicId);

    await browser.execute(() => {
      try {
        window.history.pushState({}, '', '/');
      } catch {
        window.location.replace('/');
      }
    });
    await waitForHome();

    const createPostButton = await $('[data-testid="create-post-button"]');
    await createPostButton.waitForDisplayed({ timeout: 20000 });
    await createPostButton.click();

    await selectTopicByName(topic.name);
    await selectInviteScope();

    const content = `E2E invite post ${Date.now()}`;
    const postInput = await $('[data-testid="post-input"]');
    await postInput.waitForDisplayed({ timeout: 15000 });
    await postInput.setValue(content);

    await $('[data-testid="submit-post-button"]').click();

    await browser.waitUntil(
      async () => {
        const cards = await $$('[data-testid^="post-"]');
        for (const card of cards) {
          const text = await card.getText();
          if (!text.includes(content)) {
            continue;
          }
          const badge = await card.$('[data-testid$="-scope"]');
          if (!(await badge.isExisting())) {
            continue;
          }
          const scopeValue = await badge.getAttribute('data-scope');
          if (scopeValue !== 'invite') {
            continue;
          }
          const encryptedBadge = await card.$('[data-testid$="-encrypted"]');
          if (!(await encryptedBadge.isExisting())) {
            continue;
          }
          return true;
        }
        return false;
      },
      {
        timeout: 40000,
        interval: 500,
        timeoutMsg: 'Invite scope post did not render with scope badge',
      },
    );
  });
});
