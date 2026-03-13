import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  accessControlApproveJoinRequest,
  accessControlIngestEventJson,
  accessControlListJoinRequests,
  accessControlRequestJoin,
  communityNodeListGroupKeys,
  ensureTestTopic,
  resetAppState,
  seedFriendPlusAccounts,
  switchAccount,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  openSettings,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

const TOPIC_NAME =
  process.env.E2E_COMMUNITY_NODE_FRIEND_PLUS_TOPIC_NAME ?? 'e2e-community-node-friend-plus';
const TOPIC_ID =
  process.env.E2E_COMMUNITY_NODE_FRIEND_PLUS_TOPIC_ID ?? 'kukuri:e2e-community-node-friend-plus';

const userTopicId = (pubkey: string) => `kukuri:user:${pubkey}`;

const selectFriendPlusScope = async () => {
  const scopeTrigger = await $('[data-testid="scope-selector"]');
  await scopeTrigger.waitForDisplayed({ timeout: 15000 });
  await scopeTrigger.click();
  const friendPlusItem = await $(
    '//div[@data-slot="select-item" and (contains(., "フレンド+") or contains(., "Friend+") or contains(., "好友+"))]',
  );
  await friendPlusItem.waitForDisplayed({ timeout: 10000 });
  await friendPlusItem.click();
};

describe('Community Node friend_plus flow', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('runs join.request(friend_plus) -> approve -> key.envelope -> decrypted post flow', async function () {
    this.timeout(300000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const scenario = process.env.SCENARIO;
    if (!baseUrl || scenario !== 'community-node-e2e') {
      this.skip();
      return;
    }

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Friend Plus',
      displayName: 'community-node-friend-plus',
      about: 'Community Node friend_plus join/approve/key/decrypt flow',
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

    const accessControlSwitch = await $('#community-node-access-control');
    await accessControlSwitch.waitForDisplayed({ timeout: 10000 });
    if ((await accessControlSwitch.getAttribute('data-state')) !== 'checked') {
      await accessControlSwitch.click();
    }

    const accounts = await seedFriendPlusAccounts();
    await switchAccount(accounts.requester.npub);

    const topic = await ensureTestTopic({
      name: TOPIC_NAME,
      topicId: TOPIC_ID,
    });
    expect(topic.id).toBe(TOPIC_ID);

    const joinResult = await accessControlRequestJoin({
      topic_id: topic.id,
      scope: 'friend_plus',
      target_pubkey: accounts.inviter.pubkey,
      broadcast_to_topic: false,
    });
    expect(joinResult.sent_topics).toContain(userTopicId(accounts.inviter.pubkey));

    await switchAccount(accounts.inviter.npub);
    await accessControlIngestEventJson(joinResult.event_json);

    await browser.waitUntil(
      async () => {
        const pending = await accessControlListJoinRequests();
        return pending.items.some(
          (item) =>
            item.event_id === joinResult.event_id &&
            item.topic_id === topic.id &&
            item.scope === 'friend_plus',
        );
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: 'friend_plus join.request was not listed for approval',
      },
    );

    const approveResult = await accessControlApproveJoinRequest(joinResult.event_id);
    expect(approveResult.key_envelope_event_id).toBeTruthy();

    await switchAccount(accounts.requester.npub);
    await accessControlIngestEventJson(approveResult.key_envelope_event_json);

    await browser.waitUntil(
      async () => {
        const groupKeys = await communityNodeListGroupKeys();
        return groupKeys.some((groupKey) => {
          return groupKey.topic_id === topic.id && groupKey.scope === 'friend_plus';
        });
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: 'friend_plus group key was not stored after key.envelope ingest (API check)',
      },
    );

    const syncedTopic = await ensureTestTopic({
      name: TOPIC_NAME,
      topicId: TOPIC_ID,
    });
    expect(syncedTopic.id).toBe(topic.id);
    const encodedTopicId = encodeURIComponent(syncedTopic.id);

    await browser.execute((topicId: string) => {
      const encodedTopicId = encodeURIComponent(topicId);
      try {
        window.history.pushState({}, '', `/topics/${encodedTopicId}`);
      } catch {
        window.location.replace(`/topics/${encodedTopicId}`);
      }
    }, syncedTopic.id);
    await browser.waitUntil(
      async () => (await browser.getUrl()).includes(`/topics/${encodedTopicId}`),
      {
        timeout: 20000,
        interval: 300,
        timeoutMsg: `Failed to navigate to topic route: ${syncedTopic.id}`,
      },
    );

    const createPostButton = await $('[data-testid="create-post-button"]');
    await createPostButton.waitForDisplayed({ timeout: 20000 });
    await createPostButton.click();

    await selectFriendPlusScope();

    const content = `E2E friend_plus post ${Date.now()}`;
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
          const scopeBadge = await card.$('[data-testid$="-scope"]');
          if (!(await scopeBadge.isExisting())) {
            continue;
          }
          const scopeValue = await scopeBadge.getAttribute('data-scope');
          if (scopeValue !== 'friend_plus') {
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
        timeoutMsg: 'friend_plus post did not render with decrypted content and scope badge',
      },
    );
  });
});
