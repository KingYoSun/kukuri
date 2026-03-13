import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  accessControlApproveJoinRequest,
  accessControlIssueInvite,
  accessControlListJoinRequests,
  accessControlListJoinRequestsForOwner,
  accessControlRequestJoin,
  communityNodeListGroupKeys,
  ensureTestTopic,
  joinP2PTopic,
  leaveP2PTopic,
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

type InviteContext = {
  accounts: Awaited<ReturnType<typeof seedFriendPlusAccounts>>;
  topicId: string;
  inviteEventJson: unknown;
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

const configureCommunityNodeAccessControl = async (baseUrl: string, profile: ProfileInfo) => {
  await waitForWelcome();
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
};

const prepareInviteContext = async (
  steps: BridgeStepResult[],
  stepPrefix: string,
): Promise<InviteContext> => {
  const accounts = await runBridgeStep(
    `${stepPrefix}.accounts.seed`,
    steps,
    () => seedFriendPlusAccounts(),
    (value) => ({
      requester: value.requester.npub,
      issuer: value.inviter.npub,
    }),
  );

  await switchAccount(accounts.inviter.npub);

  const issuerTopic = await runBridgeStep(
    `${stepPrefix}.issuer.ensureTopic`,
    steps,
    () => ensureTestTopic({ name: TOPIC_NAME }),
    (topic) => ({ id: topic.id, name: topic.name }),
  );

  const invite = await runBridgeStep(
    `${stepPrefix}.issuer.issueInvite`,
    steps,
    () => accessControlIssueInvite({ topic_id: issuerTopic.id }),
    () => ({ topicId: issuerTopic.id }),
  );

  return {
    accounts,
    topicId: issuerTopic.id,
    inviteEventJson: invite.invite_event_json,
  };
};

const waitForPendingInviteJoin = async (
  eventId: string,
  topicId: string,
  ownerPubkey: string,
  timeoutMsg: string,
) => {
  const deadline = Date.now() + 30_000;
  let lastCurrentCount = 0;
  let lastOwnerCount = 0;
  let ownerHasTarget = false;

  while (Date.now() < deadline) {
    const currentPending = await accessControlListJoinRequests();
    lastCurrentCount = currentPending.items.length;
    const currentHasTarget = currentPending.items.some(
      (item) => item.event_id === eventId && item.topic_id === topicId && item.scope === 'invite',
    );
    if (currentHasTarget) {
      return;
    }

    const ownerPending = await accessControlListJoinRequestsForOwner(ownerPubkey);
    lastOwnerCount = ownerPending.items.length;
    ownerHasTarget = ownerPending.items.some(
      (item) => item.event_id === eventId && item.topic_id === topicId && item.scope === 'invite',
    );
    if (ownerHasTarget) {
      throw new Error(
        `${timeoutMsg} (owner-specific query found target but current query missed it: owner_pubkey=${ownerPubkey})`,
      );
    }

    await browser.pause(500);
  }

  throw new Error(
    `${timeoutMsg} (current_count=${lastCurrentCount}, owner_count=${lastOwnerCount}, owner_has_target=${ownerHasTarget})`,
  );
};

const waitForInviteGroupKey = async (topicId: string, timeoutMsg: string) => {
  await browser.waitUntil(
    async () => {
      const groupKeys = await communityNodeListGroupKeys();
      return groupKeys.some((entry) => entry.topic_id === topicId && entry.scope === 'invite');
    },
    {
      timeout: 30000,
      interval: 500,
      timeoutMsg,
    },
  );
};

const resolveInviteScenarioBaseUrl = (context: Mocha.Context): string | null => {
  const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
  const p2pInviteReady = process.env.E2E_COMMUNITY_NODE_P2P_INVITE === '1';
  if (!baseUrl || !p2pInviteReady) {
    context.skip();
    return null;
  }
  return baseUrl;
};

describe('Community Node invite flow', () => {
  before(async () => {
    await waitForAppReady();
  });

  beforeEach(async () => {
    await resetAppState();
  });

  it('receives join.request/key.envelope automatically via relay without ingest bridge calls', async function () {
    this.timeout(300000);

    const baseUrl = resolveInviteScenarioBaseUrl(this);
    if (!baseUrl) {
      return;
    }

    const profile: ProfileInfo = {
      name: 'E2E Invite User',
      displayName: 'community-node-invite-relay-auto',
      about: 'Community Node invite flow via relay auto receive',
    };
    await configureCommunityNodeAccessControl(baseUrl, profile);

    const steps: BridgeStepResult[] = [];
    const context = await prepareInviteContext(steps, 'relayAuto');
    const issuerTopic = userTopicId(context.accounts.inviter.pubkey);
    const requesterTopic = userTopicId(context.accounts.requester.pubkey);

    await switchAccount(context.accounts.requester.npub);
    const joinResult = await runBridgeStep(
      'relayAuto.requester.requestJoin',
      steps,
      () =>
        accessControlRequestJoin({
          invite_event_json: context.inviteEventJson,
        }),
      (result) => ({
        eventId: result.event_id,
        sentTopics: result.sent_topics,
      }),
    );
    expect(joinResult.sent_topics).toContain(issuerTopic);

    await switchAccount(context.accounts.inviter.npub);
    await runBridgeStep(
      'relayAuto.issuer.refreshUserTopic',
      steps,
      async () => {
        await leaveP2PTopic(issuerTopic);
        await joinP2PTopic(issuerTopic);
      },
      () => ({ topic: issuerTopic }),
    );
    await runBridgeStep(
      'relayAuto.issuer.pendingJoinDetected',
      steps,
      async () =>
        await waitForPendingInviteJoin(
          joinResult.event_id,
          context.topicId,
          context.accounts.inviter.pubkey,
          'invite join.request was not auto-received via relay',
        ),
      () => ({ eventId: joinResult.event_id, topicId: context.topicId }),
    );

    const approveResult = await runBridgeStep(
      'relayAuto.issuer.approveJoinRequest',
      steps,
      () => accessControlApproveJoinRequest(joinResult.event_id),
      (result) => ({
        eventId: result.event_id,
        keyEnvelopeEventId: result.key_envelope_event_id,
      }),
    );
    expect(approveResult.key_envelope_event_id).toBeTruthy();

    await switchAccount(context.accounts.requester.npub);
    await runBridgeStep(
      'relayAuto.requester.refreshUserTopic',
      steps,
      async () => {
        await leaveP2PTopic(requesterTopic);
        await joinP2PTopic(requesterTopic);
      },
      () => ({ topic: requesterTopic }),
    );
    await runBridgeStep(
      'relayAuto.requester.keyEnvelopeDetected',
      steps,
      async () =>
        await waitForInviteGroupKey(
          context.topicId,
          'invite key.envelope was not auto-received via relay',
        ),
      () => ({ topicId: context.topicId }),
    );

    const topicByTopicId = await runBridgeStep(
      'relayAuto.requester.ensureTopicByTopicId',
      steps,
      () => ensureTestTopic({ name: TOPIC_NAME, topicId: context.topicId }),
      (topic) => ({ id: topic.id, name: topic.name }),
    );
    expect(topicByTopicId.id).toBe(context.topicId);
  });

  it('replays join.request and key.envelope after issuer/requester resume', async function () {
    this.timeout(300000);

    const baseUrl = resolveInviteScenarioBaseUrl(this);
    if (!baseUrl) {
      return;
    }

    const profile: ProfileInfo = {
      name: 'E2E Invite Resume',
      displayName: 'community-node-invite-resume',
      about: 'Community Node invite replay on issuer/requester resume',
    };
    await configureCommunityNodeAccessControl(baseUrl, profile);

    const steps: BridgeStepResult[] = [];
    const context = await prepareInviteContext(steps, 'resumeFlow');
    const issuerTopic = userTopicId(context.accounts.inviter.pubkey);
    const requesterTopic = userTopicId(context.accounts.requester.pubkey);

    await runBridgeStep(
      'resumeFlow.issuer.goOffline.leaveUserTopic',
      steps,
      () => leaveP2PTopic(issuerTopic),
      () => ({ topic: issuerTopic }),
    );

    await switchAccount(context.accounts.requester.npub);
    const joinResult = await runBridgeStep(
      'resumeFlow.requester.requestJoin',
      steps,
      () =>
        accessControlRequestJoin({
          invite_event_json: context.inviteEventJson,
        }),
      (result) => ({
        eventId: result.event_id,
        sentTopics: result.sent_topics,
      }),
    );
    expect(joinResult.sent_topics).toContain(issuerTopic);

    await switchAccount(context.accounts.inviter.npub);
    await runBridgeStep(
      'resumeFlow.issuer.resume.joinUserTopic',
      steps,
      () => joinP2PTopic(issuerTopic),
      () => ({ topic: issuerTopic }),
    );
    await runBridgeStep(
      'resumeFlow.issuer.pendingJoinReplayed',
      steps,
      async () =>
        await waitForPendingInviteJoin(
          joinResult.event_id,
          context.topicId,
          context.accounts.inviter.pubkey,
          'invite join.request was not replayed after issuer resume',
        ),
      () => ({ eventId: joinResult.event_id, topicId: context.topicId }),
    );

    await switchAccount(context.accounts.requester.npub);
    await runBridgeStep(
      'resumeFlow.requester.goOffline.leaveUserTopic',
      steps,
      () => leaveP2PTopic(requesterTopic),
      () => ({ topic: requesterTopic }),
    );

    await switchAccount(context.accounts.inviter.npub);
    const approveResult = await runBridgeStep(
      'resumeFlow.issuer.approveJoinRequest',
      steps,
      () => accessControlApproveJoinRequest(joinResult.event_id),
      (result) => ({
        eventId: result.event_id,
        keyEnvelopeEventId: result.key_envelope_event_id,
      }),
    );
    expect(approveResult.key_envelope_event_id).toBeTruthy();

    await switchAccount(context.accounts.requester.npub);
    await runBridgeStep(
      'resumeFlow.requester.resume.joinUserTopic',
      steps,
      () => joinP2PTopic(requesterTopic),
      () => ({ topic: requesterTopic }),
    );
    await runBridgeStep(
      'resumeFlow.requester.keyEnvelopeReplayed',
      steps,
      async () =>
        await waitForInviteGroupKey(
          context.topicId,
          'invite key.envelope was not replayed after requester resume',
        ),
      () => ({ topicId: context.topicId }),
    );

    const topicByTopicId = await runBridgeStep(
      'resumeFlow.requester.ensureTopicByTopicId',
      steps,
      () => ensureTestTopic({ name: TOPIC_NAME, topicId: context.topicId }),
      (topic) => ({ id: topic.id, name: topic.name }),
    );
    expect(topicByTopicId.id).toBe(context.topicId);
  });
});
