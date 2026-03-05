import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  accessControlApproveJoinRequest,
  accessControlIngestEventJson,
  accessControlIssueInvite,
  accessControlListJoinRequests,
  accessControlRequestJoin,
  communityNodeListGroupKeys,
  ensureTestTopic,
  getTopicSnapshot,
  resetAppState,
  seedFriendPlusAccounts,
  switchAccount,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  openSettings,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

const TOPIC_NAME = process.env.E2E_COMMUNITY_NODE_TOPIC_NAME ?? 'e2e-community-node-invite';

const userTopicId = (pubkey: string) => `kukuri:user:${pubkey}`;

type BridgeStepResult = {
  step: string;
  status: 'ok' | 'failed';
  detail?: unknown;
  error?: string;
};

const toErrorMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message;
  }
  if (error && typeof error === 'object') {
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }
  return String(error);
};

const snapshotSummary = (snapshot: Awaited<ReturnType<typeof getTopicSnapshot>>) => ({
  topicCount: snapshot.topics.length,
  joinedTopicCount: snapshot.joinedTopics.length,
  pendingTopicCount: snapshot.pendingTopics.length,
});

const runBridgeStep = async <T>(
  step: string,
  steps: BridgeStepResult[],
  fn: () => Promise<T>,
  summarize?: (value: T) => unknown,
): Promise<T> => {
  try {
    const value = await fn();
    steps.push({
      step,
      status: 'ok',
      detail: summarize ? summarize(value) : null,
    });
    return value;
  } catch (error) {
    const message = toErrorMessage(error);
    steps.push({
      step,
      status: 'failed',
      error: message,
    });
    throw new Error(
      [
        `Bridge step failed: ${step}`,
        message,
        `Bridge steps: ${JSON.stringify(steps, null, 2)}`,
      ].join('\n'),
      { cause: error },
    );
  }
};

describe('Community Node invite flow', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('isolates bridge calls around ensureTestTopic after invite join request', async function () {
    this.timeout(300000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const p2pInviteReady = process.env.E2E_COMMUNITY_NODE_P2P_INVITE === '1';
    if (!baseUrl || !p2pInviteReady) {
      this.skip();
      return;
    }

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

    const accessControlSwitch = await $('#community-node-access-control');
    await accessControlSwitch.waitForDisplayed({ timeout: 10000 });
    if ((await accessControlSwitch.getAttribute('data-state')) !== 'checked') {
      await accessControlSwitch.click();
    }

    const steps: BridgeStepResult[] = [];
    const accounts = await runBridgeStep(
      'invite.accounts.seed',
      steps,
      () => seedFriendPlusAccounts(),
      (value) => ({
        requester: value.requester.npub,
        issuer: value.inviter.npub,
      }),
    );

    await switchAccount(accounts.inviter.npub);

    const issuerTopic = await runBridgeStep(
      'invite.issuer.ensureTopic',
      steps,
      () => ensureTestTopic({ name: TOPIC_NAME }),
      (topic) => ({ id: topic.id, name: topic.name }),
    );
    const topicId = issuerTopic.id;

    const invite = await runBridgeStep(
      'invite.issuer.issueInvite',
      steps,
      () => accessControlIssueInvite({ topic_id: topicId }),
      () => ({ topicId }),
    );

    await switchAccount(accounts.requester.npub);

    const joinResult = await runBridgeStep(
      'invite.requester.requestJoin',
      steps,
      () =>
        accessControlRequestJoin({
          invite_event_json: invite.invite_event_json,
        }),
      (result) => ({
        eventId: result.event_id,
        sentTopics: result.sent_topics,
      }),
    );
    expect(joinResult.sent_topics).toContain(userTopicId(accounts.inviter.pubkey));

    await switchAccount(accounts.inviter.npub);
    await runBridgeStep('invite.issuer.ingestJoinRequest', steps, () =>
      accessControlIngestEventJson(joinResult.event_json),
    );

    await runBridgeStep(
      'invite.issuer.pendingJoinDetected',
      steps,
      async () => {
        await browser.waitUntil(
          async () => {
            const pending = await accessControlListJoinRequests();
            return pending.items.some(
              (item) =>
                item.event_id === joinResult.event_id &&
                item.topic_id === topicId &&
                item.scope === 'invite',
            );
          },
          {
            timeout: 30000,
            interval: 500,
            timeoutMsg: 'invite join.request was not listed for approval',
          },
        );
        return true;
      },
      () => ({ eventId: joinResult.event_id, topicId }),
    );

    const approveResult = await runBridgeStep(
      'invite.issuer.approveJoinRequest',
      steps,
      () => accessControlApproveJoinRequest(joinResult.event_id),
      (result) => ({
        eventId: result.event_id,
        keyEnvelopeEventId: result.key_envelope_event_id,
      }),
    );
    expect(approveResult.key_envelope_event_id).toBeTruthy();

    await switchAccount(accounts.requester.npub);
    await runBridgeStep('invite.requester.ingestKeyEnvelope', steps, () =>
      accessControlIngestEventJson(approveResult.key_envelope_event_json),
    );

    const keyEnvelopeDetected = await runBridgeStep(
      'invite.keyEnvelopeDetected',
      steps,
      async () => {
        await browser.waitUntil(
          async () => {
            const groupKeys = await communityNodeListGroupKeys();
            return groupKeys.some(
              (entry) => entry.topic_id === topicId && entry.scope === 'invite',
            );
          },
          {
            timeout: 30000,
            interval: 500,
            timeoutMsg: 'Key envelope was not stored after issuer approval',
          },
        );
        return true;
      },
      (detected) => ({ topicId, keyEnvelopeDetected: detected }),
    );
    expect(keyEnvelopeDetected).toBe(true);

    const beforeSnapshot = await runBridgeStep(
      'snapshot.before.ensure',
      steps,
      () => getTopicSnapshot(),
      snapshotSummary,
    );
    expect(beforeSnapshot.topics.length).toBeGreaterThanOrEqual(0);

    const topicByName = await runBridgeStep(
      'ensure.byName',
      steps,
      () => ensureTestTopic({ name: TOPIC_NAME }),
      (topic) => ({ id: topic.id, name: topic.name }),
    );
    expect(topicByName.name).toBe(TOPIC_NAME);

    await runBridgeStep('snapshot.after.byName', steps, () => getTopicSnapshot(), snapshotSummary);

    const topicByTopicId = await runBridgeStep(
      'ensure.byTopicId',
      steps,
      () => ensureTestTopic({ name: TOPIC_NAME, topicId }),
      (topic) => ({ id: topic.id, name: topic.name }),
    );
    expect(topicByTopicId.id).toBe(topicId);

    const afterSnapshot = await runBridgeStep(
      'snapshot.after.byTopicId',
      steps,
      () => getTopicSnapshot(),
      snapshotSummary,
    );
    expect(afterSnapshot.topics.length).toBeGreaterThan(0);
  });
});
