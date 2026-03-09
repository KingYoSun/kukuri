import { $, browser, expect } from '@wdio/globals';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  getP2PMessageSnapshot,
  getP2PStatus,
  getRelayStatusSnapshot,
  resetAppState,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  startCreateAccountFlow,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';
import {
  enqueuePeerHarnessPublishCommand,
  waitForPeerHarnessCommandResult,
  waitForPeerHarnessSummary,
} from '../helpers/peerHarness';

const DEFAULT_PUBLIC_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_OUTPUT_GROUP = 'community-node-e2e';
const DEFAULT_PUBLISHER_PEER = 'peer-client-2';
const DEFAULT_PROFILE_NAME = 'community-node-peer-publisher-profile';
const DEFAULT_PROFILE_PICTURE =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgwJ/l7hR9QAAAABJRU5ErkJggg==';

const profile: ProfileInfo = {
  name: 'E2E Profile Resolution',
  displayName: 'community-node-profile-resolution',
  about: 'Community Node profile resolution without explicit profile update',
};

interface RenderedAuthorSnapshot {
  testId: string;
  authorName: string | null;
  avatarSrc: string | null;
  text: string;
}

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

const waitForThreadRoute = async (topicId: string, threadUuid: string): Promise<void> => {
  await browser.waitUntil(
    async () =>
      decodeURIComponent(await browser.getUrl()).includes(
        `/topics/${topicId}/threads/${threadUuid}`,
      ),
    {
      timeout: 30000,
      interval: 500,
      timeoutMsg: `Thread route did not open: ${topicId} / ${threadUuid}`,
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
    async () => {
      const peerCount = await $('[data-testid="topic-mesh-peer-count"]');
      const raw = await peerCount.getAttribute('data-count');
      return Number(raw ?? '0') > 0;
    },
    {
      timeout: 120000,
      interval: 1000,
      timeoutMsg: 'Topic mesh peer count did not become positive',
    },
  );
};

const waitForConnectedNostrRelay = async (): Promise<void> => {
  let latest: {
    relays: Array<{ url: string; status: string }>;
    error: string | null;
  } | null = null;

  await browser.waitUntil(
    async () => {
      latest = await getRelayStatusSnapshot();
      return latest.error === null && latest.relays.some((relay) => relay.status === 'connected');
    },
    {
      timeout: 30000,
      interval: 1000,
      timeoutMsg: `Nostr relay did not become connected: ${JSON.stringify(latest)}`,
    },
  );
};

const readRenderedAuthor = async (
  containerSelector: string,
  contentNeedle: string,
): Promise<RenderedAuthorSnapshot | null> => {
  return await browser.execute(
    (selector: string, needle: string) => {
      const containers = Array.from(document.querySelectorAll<HTMLElement>(selector));
      const target = containers.find((element) => (element.innerText ?? '').includes(needle));
      if (!target) {
        return null;
      }

      const authorName =
        target.querySelector<HTMLElement>('[data-testid$="-author-name"]')?.innerText?.trim() ??
        null;
      const avatarSrc =
        target
          .querySelector<HTMLElement>('[data-testid$="-author-avatar"]')
          ?.getAttribute('data-avatar-src') ??
        target
          .querySelector<HTMLImageElement>('[data-testid$="-author-avatar"] img')
          ?.getAttribute('src') ??
        null;

      return {
        testId: target.getAttribute('data-testid') ?? '',
        authorName,
        avatarSrc,
        text: target.innerText ?? '',
      };
    },
    containerSelector,
    contentNeedle,
  );
};

const waitForRenderedAuthor = async (options: {
  containerSelector: string;
  contentNeedle: string;
  expectedAuthorName: string;
  expectedAvatarSrc: string;
  description: string;
  timeoutMs?: number;
}): Promise<RenderedAuthorSnapshot> => {
  const timeoutMs = options.timeoutMs ?? 180000;
  let latest: RenderedAuthorSnapshot | null = null;

  await browser.waitUntil(
    async () => {
      latest = await readRenderedAuthor(options.containerSelector, options.contentNeedle);
      return (
        latest?.authorName === options.expectedAuthorName &&
        latest?.avatarSrc === options.expectedAvatarSrc
      );
    },
    {
      timeout: timeoutMs,
      interval: 1000,
      timeoutMsg: `${options.description}; latest=${JSON.stringify(latest)}`,
    },
  );

  if (!latest) {
    throw new Error(`${options.description}; latest snapshot is unavailable`);
  }

  return latest;
};

const waitForP2PMarker = async (topicId: string, marker: string): Promise<void> => {
  await browser.waitUntil(
    async () => {
      const snapshot = await getP2PMessageSnapshot(topicId);
      return snapshot.recentContents.some((content) => content.includes(marker));
    },
    {
      timeout: 180000,
      interval: 1000,
      timeoutMsg: `P2P snapshot did not contain expected content: ${marker}`,
    },
  );
};

describe('Community Node profile resolution', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('renders author display name and avatar in timeline/thread without relying on profile update push', async function () {
    this.timeout(480000);

    if (process.env.SCENARIO !== 'community-node-e2e') {
      this.skip();
      return;
    }

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl) {
      throw new Error('E2E_COMMUNITY_NODE_URL is not set');
    }

    const outputGroup = process.env.KUKURI_PEER_OUTPUT_GROUP?.trim() || DEFAULT_OUTPUT_GROUP;
    const publisherPeer =
      process.env.E2E_COMMUNITY_NODE_PUBLISHER_PEER?.trim() || DEFAULT_PUBLISHER_PEER;
    const expectedAuthorName =
      process.env.KUKURI_PEER_PROFILE_NAME_2?.trim() || DEFAULT_PROFILE_NAME;
    const expectedAvatarSrc =
      process.env.KUKURI_PEER_PROFILE_PICTURE_2?.trim() || DEFAULT_PROFILE_PICTURE;

    const startupSummary = await waitForPeerHarnessSummary({
      peerName: publisherPeer,
      outputGroup,
      timeoutMs: 120000,
      description: 'publisher peer should publish startup metadata before listener joins',
      predicate: (summary) => (summary.stats?.metadata_published_count ?? 0) >= 1,
    });
    const startupMetadataCount = startupSummary.stats?.metadata_published_count ?? 0;
    const startupPublishedCount = startupSummary.stats?.published_count ?? 0;
    const startupPeerJoinedEvents = startupSummary.stats?.peer_joined_events ?? 0;

    await waitForWelcome();
    await startCreateAccountFlow();
    await completeProfileSetup(profile);
    await waitForHome();

    await runCommunityNodeAuthFlow(baseUrl);

    const consentButton = await $('[data-testid="community-node-accept-consents"]');
    await browser.waitUntil(async () => await consentButton.isEnabled(), {
      timeout: 20000,
      interval: 300,
      timeoutMsg: 'Consent button did not become enabled',
    });
    await consentButton.click();
    await waitForConnectedNostrRelay();

    const topicButton = await $(`[data-testid="topic-${DEFAULT_PUBLIC_TOPIC_ID}"]`);
    await topicButton.waitForDisplayed({ timeout: 30000 });
    await topicButton.click();
    await waitForTopicRoute(DEFAULT_PUBLIC_TOPIC_ID);

    const realtimeToggle = await $('[data-testid="timeline-mode-toggle-realtime"]');
    await realtimeToggle.waitForDisplayed({ timeout: 15000 });
    await realtimeToggle.click();

    const joinButton = await $('[data-testid="topic-mesh-join-button"]');
    if (await joinButton.isExisting()) {
      await joinButton.waitForDisplayed({ timeout: 20000 });
      await joinButton.click();
    }

    await waitForActiveTopic(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForBackendPeerConnectivity(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForTopicMeshPeerCount();

    const postJoinSummary = await waitForPeerHarnessSummary({
      peerName: publisherPeer,
      outputGroup,
      timeoutMs: 120000,
      description:
        'publisher peer should observe the receiver join without standalone metadata republish',
      predicate: (summary) => (summary.stats?.peer_joined_events ?? 0) > startupPeerJoinedEvents,
    });

    const metadataCountAfterJoin = postJoinSummary.stats?.metadata_published_count ?? 0;
    const publishedCountAfterJoin = postJoinSummary.stats?.published_count ?? 0;
    expect(metadataCountAfterJoin - startupMetadataCount).toBe(
      publishedCountAfterJoin - startupPublishedCount,
    );

    const marker = `community-node-profile-resolution-${Date.now()}`;
    const { commandId } = enqueuePeerHarnessPublishCommand({
      peerName: publisherPeer,
      outputGroup,
      topicId: DEFAULT_PUBLIC_TOPIC_ID,
      content: marker,
    });
    const commandResult = await waitForPeerHarnessCommandResult({
      peerName: publisherPeer,
      outputGroup,
      commandId,
      timeoutMs: 120000,
      description: `publisher peer should publish ${marker}`,
    });
    const metadataCountAfterCommand = commandResult.metadata_published_count ?? 0;
    const publishedCountAfterCommand = commandResult.published_count ?? 0;
    expect(metadataCountAfterCommand).toBeGreaterThan(metadataCountAfterJoin);
    expect(metadataCountAfterCommand - metadataCountAfterJoin).toBe(
      publishedCountAfterCommand - publishedCountAfterJoin,
    );

    await waitForP2PMarker(DEFAULT_PUBLIC_TOPIC_ID, marker);

    const timelineSnapshot = await waitForRenderedAuthor({
      containerSelector: '[data-testid^="timeline-thread-card-"]',
      contentNeedle: marker,
      expectedAuthorName,
      expectedAvatarSrc,
      description:
        'timeline card did not resolve author display name/avatar from post-time metadata propagation',
    });

    const threadUuid = timelineSnapshot.testId.replace('timeline-thread-card-', '');
    if (!threadUuid) {
      throw new Error(
        `Failed to resolve thread uuid from timeline snapshot: ${timelineSnapshot.testId}`,
      );
    }

    const openThreadButton = await $(`[data-testid="timeline-thread-open-${threadUuid}"]`);
    await openThreadButton.waitForClickable({ timeout: 20000 });
    await openThreadButton.click();

    await waitForThreadRoute(DEFAULT_PUBLIC_TOPIC_ID, threadUuid);
    await $('[data-testid="thread-detail-title"]').waitForDisplayed({ timeout: 20000 });

    const threadSnapshot = await waitForRenderedAuthor({
      containerSelector: '[data-testid^="forum-thread-post-"]',
      contentNeedle: marker,
      expectedAuthorName,
      expectedAvatarSrc,
      description:
        'thread detail did not resolve author display name/avatar from post-time metadata propagation',
    });

    expect(timelineSnapshot.authorName).toBe(expectedAuthorName);
    expect(timelineSnapshot.avatarSrc).toBe(expectedAvatarSrc);
    expect(threadSnapshot.authorName).toBe(expectedAuthorName);
    expect(threadSnapshot.avatarSrc).toBe(expectedAvatarSrc);
  });
});
