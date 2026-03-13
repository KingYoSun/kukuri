import { $, browser, expect } from '@wdio/globals';

import {
  getAuthSnapshot,
  getP2PStatus,
  getRelayStatusSnapshot,
  resetAppState,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  openSettings,
  startCreateAccountFlow,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';
import { waitForPeerHarnessSummary } from '../helpers/peerHarness';
import {
  expectNoToastMatching,
  waitForToastsToClear,
} from '../helpers/toasts';
import { waitForAppReady } from '../helpers/waitForAppReady';

const DEFAULT_LISTENER_PEER = 'peer-client-1';
const DEFAULT_OUTPUT_GROUP = 'community-node-e2e';
const DEFAULT_PUBLIC_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';

const initialProfile: ProfileInfo = {
  name: 'E2E Profile Initial',
  displayName: 'community-node-profile-initial',
  about: 'Community Node profile propagation initial state',
};

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

const normalizeRelayUrl = (value: string): string => value.trim().replace(/\/+$/, '');

const parseIntAttribute = async (selector: string, attribute = 'data-count'): Promise<number> => {
  const element = await $(selector);
  if (!(await element.isExisting())) {
    return 0;
  }
  const raw = await element.getAttribute(attribute);
  const parsed = Number(raw ?? '0');
  return Number.isFinite(parsed) ? parsed : 0;
};

const waitForTopicRoute = async (topicId: string): Promise<void> => {
  const encodedTopicId = encodeURIComponent(topicId);
  await browser.waitUntil(
    async () => {
      const currentUrl = decodeURIComponent(await browser.getUrl());
      return (
        currentUrl.includes(`/topics/${topicId}`) ||
        currentUrl.includes(`/topics/${encodedTopicId}`)
      );
    },
    {
      timeout: 30000,
      interval: 500,
      timeoutMsg: `Topic route did not open: ${topicId}`,
    },
  );
};

const waitForActiveTopic = async (topicId: string): Promise<void> => {
  await browser.waitUntil(
    async () => {
      const status = await getP2PStatus();
      return status.active_topics.some((topic) => topic.topic_id === topicId);
    },
    {
      timeout: 120000,
      interval: 1000,
      timeoutMsg: `P2P active topic did not appear for ${topicId}`,
    },
  );
};

const waitForBackendPeerConnectivity = async (topicId: string): Promise<void> => {
  await browser.waitUntil(
    async () => {
      const status = await getP2PStatus();
      const topic = status.active_topics.find((entry) => entry.topic_id === topicId);
      return Boolean(topic && topic.peer_count > 0);
    },
    {
      timeout: 120000,
      interval: 1000,
      timeoutMsg: `P2P backend peer count did not become positive for ${topicId}`,
    },
  );
};

const waitForTopicMeshPeerCount = async (): Promise<void> => {
  await browser.waitUntil(
    async () => (await parseIntAttribute('[data-testid="topic-mesh-peer-count"]')) > 0,
    {
      timeout: 120000,
      interval: 1000,
      timeoutMsg: 'Topic mesh peer count did not become positive',
    },
  );
};

const waitForExpectedRelayConnection = async (expectedRelayUrl: string): Promise<void> => {
  const normalizedExpected = normalizeRelayUrl(expectedRelayUrl);
  let latestSnapshot:
    | {
        relays: Array<{ url: string; status: string }>;
        error: string | null;
        lastFetchedAt: number | null;
      }
    | null = null;

  await browser.waitUntil(
    async () => {
      latestSnapshot = await getRelayStatusSnapshot();
      return latestSnapshot.relays.some(
        (relay) =>
          normalizeRelayUrl(relay.url) === normalizedExpected &&
          relay.status.toLowerCase() === 'connected',
      );
    },
    {
      timeout: 120000,
      interval: 1000,
      timeoutMsg: `Expected Nostr relay did not connect: ${normalizedExpected}`,
    },
  );

  if (!latestSnapshot) {
    throw new Error(`Relay status snapshot was unavailable for ${normalizedExpected}`);
  }
};

describe('Community Node profile propagation', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('propagates updated profile metadata without false failure toasts', async function () {
    this.timeout(360000);

    if (process.env.SCENARIO !== 'community-node-e2e') {
      this.skip();
      return;
    }

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl) {
      throw new Error('E2E_COMMUNITY_NODE_URL is not set');
    }

    const expectedRelayUrl = resolveEnvString(
      process.env.E2E_COMMUNITY_NODE_EXPECTED_RELAY_URL,
      'ws://127.0.0.1:18082/relay',
    );
    const listenerPeer = resolveEnvString(
      process.env.E2E_COMMUNITY_NODE_LISTENER_PEER,
      DEFAULT_LISTENER_PEER,
    );
    const outputGroup = resolveEnvString(process.env.KUKURI_PEER_OUTPUT_GROUP, DEFAULT_OUTPUT_GROUP);

    await waitForWelcome();
    await startCreateAccountFlow();
    await completeProfileSetup(initialProfile);
    await waitForHome();
    await expectNoToastMatching({
      patterns: [
        /Profile saved with partial failures/i,
        /Profile setup failed/i,
        /プロフィールの保存中に一部失敗しました/,
        /プロフィール設定に失敗しました/,
      ],
      durationMs: 6000,
      description: 'initial profile setup should not emit failure or warning toast',
    });

    await runCommunityNodeAuthFlow(baseUrl);
    await waitForExpectedRelayConnection(expectedRelayUrl);
    await waitForToastsToClear().catch(() => {});

    const consentButton = await $('[data-testid="community-node-accept-consents"]');
    await browser.waitUntil(async () => await consentButton.isEnabled(), {
      timeout: 20000,
      interval: 300,
      timeoutMsg: 'Consent button did not become enabled',
    });
    await consentButton.click();

    const topicButton = await $(`[data-testid="topic-${DEFAULT_PUBLIC_TOPIC_ID}"]`);
    await topicButton.waitForDisplayed({ timeout: 30000 });
    await topicButton.click();
    await waitForTopicRoute(DEFAULT_PUBLIC_TOPIC_ID);

    const joinButton = await $('[data-testid="topic-mesh-join-button"]');
    if (await joinButton.isExisting()) {
      await joinButton.waitForDisplayed({ timeout: 20000 });
      await joinButton.click();
    }

    await waitForActiveTopic(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForBackendPeerConnectivity(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForTopicMeshPeerCount();

    const baselineListenerSummary = await waitForPeerHarnessSummary({
      peerName: listenerPeer,
      outputGroup,
      timeoutMs: 120000,
      description: 'listener peer summary should be available before profile update',
      predicate: () => true,
    });
    const baselineReceivedCount = baselineListenerSummary.stats?.received_count ?? 0;

    const updatedProfile = {
      name: `profile-sync-name-${Date.now()}`,
      displayName: `profile-sync-display-${Date.now()}`,
      about: `profile-sync-about-${Date.now()}`,
    };

    await openSettings();
    const openProfileDialogButton = await $('[data-testid="open-profile-dialog"]');
    await openProfileDialogButton.waitForClickable({ timeout: 15000 });
    await openProfileDialogButton.click();

    const profileForm = await $('[data-testid="profile-form"]');
    await profileForm.waitForDisplayed({ timeout: 15000 });
    await $('[data-testid="profile-name"]').setValue(updatedProfile.name);
    await $('[data-testid="profile-display-name"]').setValue(updatedProfile.displayName);
    await $('[data-testid="profile-about"]').setValue(updatedProfile.about);
    await $('[data-testid="profile-submit"]').click();
    await profileForm.waitForDisplayed({ reverse: true, timeout: 20000 });

    await expectNoToastMatching({
      patterns: [
        /Profile saved with partial failures/i,
        /Profile update failed/i,
        /プロフィールの保存中に一部失敗しました/,
        /プロフィールの更新に失敗しました/,
      ],
      durationMs: 6000,
      description: 'profile update should not emit failure toast after community node auth',
    });

    await browser.waitUntil(
      async () => {
        const snapshot = await getAuthSnapshot();
        return snapshot.currentUser?.displayName === updatedProfile.displayName;
      },
      {
        timeout: 20000,
        interval: 300,
        timeoutMsg: 'Updated profile did not propagate into auth snapshot',
      },
    );

    const listenerSummary = await waitForPeerHarnessSummary({
      peerName: listenerPeer,
      outputGroup,
      timeoutMs: 180000,
      description: 'listener peer should receive propagated profile metadata',
      predicate: (summary) =>
        (summary.stats?.received_count ?? 0) > baselineReceivedCount &&
        (summary.stats?.recent_contents ?? []).some(
          (content) =>
            content.includes(updatedProfile.displayName) && content.includes(updatedProfile.about),
        ),
    });

    expect(
      (listenerSummary.stats?.recent_contents ?? []).some(
        (content) =>
          content.includes(updatedProfile.displayName) && content.includes(updatedProfile.about),
      ),
    ).toBe(true);
  });
});
