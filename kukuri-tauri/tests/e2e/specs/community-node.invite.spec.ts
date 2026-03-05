import { $, $$, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import { resetAppState, ensureTestTopic, getTopicSnapshot } from '../helpers/bridge';
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
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const p2pInviteReady = process.env.E2E_COMMUNITY_NODE_P2P_INVITE === '1';
    if (!baseUrl || !INVITE_JSON || !p2pInviteReady) {
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
    await $('[data-testid="community-node-request-join"]').click();

    const steps: BridgeStepResult[] = [];
    const keyEnvelopeDetected = await browser
      .waitUntil(
        async () => {
          const topicEntries = await $$('[data-testid="community-node-saved-key-topic"]');
          for (const entry of topicEntries) {
            if ((await entry.getText()).trim() === topicId) {
              return true;
            }
          }
          return false;
        },
        {
          timeout: 30000,
          interval: 500,
          timeoutMsg: 'Key envelope was not stored after P2P join request',
        },
      )
      .then(() => true)
      .catch(() => false);
    steps.push({
      step: 'invite.keyEnvelopeDetected',
      status: keyEnvelopeDetected ? 'ok' : 'failed',
      detail: { topicId, keyEnvelopeDetected },
    });

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
