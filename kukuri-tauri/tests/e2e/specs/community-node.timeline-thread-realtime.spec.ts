import { $, browser, expect } from '@wdio/globals';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  getP2PMessageSnapshot,
  getP2PStatus,
  getPostStoreSnapshot,
  getTopicTimelineQuerySnapshot,
  resetAppState,
  getTimelineUpdateMode,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  startCreateAccountFlow,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

const DEFAULT_PUBLIC_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_PUBLISH_PREFIX = 'community-node-peer-publisher';

const profile: ProfileInfo = {
  name: 'E2E Timeline Realtime',
  displayName: 'community-node-timeline-realtime',
  about: 'Community Node realtime timeline propagation regression coverage',
};

interface TimelineThreadSnapshot {
  ids: string[];
  count: number;
  textDigest: string;
  containsNeedle: boolean;
}

type P2PMessageSnapshot = Awaited<ReturnType<typeof getP2PMessageSnapshot>>;

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

const waitForRecentMeshMessage = async (needle: string): Promise<void> => {
  await browser.waitUntil(
    async () =>
      await browser.execute((contentNeedle: string) => {
        return Array.from(
          document.querySelectorAll<HTMLElement>('[data-testid="topic-mesh-recent-message"]'),
        ).some((element) => (element.innerText ?? '').includes(contentNeedle));
      }, needle),
    {
      timeout: 180000,
      interval: 1000,
      timeoutMsg: `Topic mesh did not receive expected message: ${needle}`,
    },
  );
};

const waitForBodyText = async (needle: string, timeoutMs: number): Promise<boolean> => {
  try {
    await browser.waitUntil(
      async () =>
        await browser.execute((contentNeedle: string) => {
          return (document.body?.innerText ?? '').includes(contentNeedle);
        }, needle),
      {
        timeout: timeoutMs,
        interval: 500,
      },
    );
    return true;
  } catch {
    return false;
  }
};

const getTimelineThreadSnapshot = async (needle: string): Promise<TimelineThreadSnapshot> => {
  return await browser.execute((contentNeedle: string) => {
    const cards = Array.from(
      document.querySelectorAll<HTMLElement>('[data-testid^="timeline-thread-card-"]'),
    );
    const ids = cards
      .map((card) => card.getAttribute('data-testid') ?? '')
      .filter((value) => value.length > 0);
    const texts = cards.map((card) => card.innerText ?? '');
    return {
      ids,
      count: cards.length,
      textDigest: texts.join('\n----\n'),
      containsNeedle: texts.some((text) => text.includes(contentNeedle)),
    };
  }, needle);
};

const hasTimelineSnapshotChanged = (
  baseline: TimelineThreadSnapshot,
  current: TimelineThreadSnapshot,
): boolean => {
  if (baseline.count !== current.count) {
    return true;
  }
  if (baseline.ids.length !== current.ids.length) {
    return true;
  }
  for (let index = 0; index < baseline.ids.length; index += 1) {
    if (baseline.ids[index] !== current.ids[index]) {
      return true;
    }
  }
  return baseline.textDigest !== current.textDigest;
};

const waitForTimelineSnapshotChange = async (
  baseline: TimelineThreadSnapshot,
  needle: string,
  timeoutMs: number,
): Promise<{ changed: boolean; snapshot: TimelineThreadSnapshot }> => {
  let latest = await getTimelineThreadSnapshot(needle);
  try {
    await browser.waitUntil(
      async () => {
        latest = await getTimelineThreadSnapshot(needle);
        return hasTimelineSnapshotChanged(baseline, latest);
      },
      {
        timeout: timeoutMs,
        interval: 500,
      },
    );
    return { changed: true, snapshot: latest };
  } catch {
    return { changed: false, snapshot: latest };
  }
};

const waitForP2PMessageAdvance = async (
  topicId: string,
  publishPrefix: string,
  baseline: P2PMessageSnapshot,
  timeoutMs: number,
): Promise<P2PMessageSnapshot> => {
  const baselineIds = new Set(baseline.recentMessageIds);
  let latest = await getP2PMessageSnapshot(topicId);
  try {
    await browser.waitUntil(
      async () => {
        latest = await getP2PMessageSnapshot(topicId);
        const hasNewId = latest.recentMessageIds.some((messageId) => !baselineIds.has(messageId));
        const hasPrefix = latest.recentContents.some((content) => content.includes(publishPrefix));
        return (latest.count > baseline.count || hasNewId) && hasPrefix;
      },
      {
        timeout: timeoutMs,
        interval: 1000,
      },
    );
    return latest;
  } catch {
    throw new Error(
      `P2P snapshot did not advance for prefix=${publishPrefix} (baselineCount=${baseline.count}, latestCount=${latest.count})`,
    );
  }
};

const pickNewP2PContentMarker = (
  baseline: P2PMessageSnapshot,
  latest: P2PMessageSnapshot,
  publishPrefix: string,
): string | null => {
  const baselineIds = new Set(baseline.recentMessageIds);
  const baselineContents = new Set(baseline.recentContents);

  for (let index = 0; index < latest.recentMessageIds.length; index += 1) {
    const messageId = latest.recentMessageIds[index];
    if (baselineIds.has(messageId)) {
      continue;
    }
    const candidate = latest.recentContents[index];
    if (candidate?.includes(publishPrefix)) {
      return candidate;
    }
  }

  return (
    latest.recentContents.find(
      (content) => content.includes(publishPrefix) && !baselineContents.has(content),
    ) ?? null
  );
};

describe('Community Node realtime timeline propagation', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('renders propagated peer posts in realtime timeline without requiring refresh', async function () {
    this.timeout(480000);

    if (process.env.SCENARIO !== 'community-node-e2e') {
      this.skip();
      return;
    }

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    if (!baseUrl) {
      throw new Error('E2E_COMMUNITY_NODE_URL is not set');
    }

    const publishPrefix =
      process.env.E2E_MULTI_PEER_PUBLISH_PREFIX?.trim() ||
      process.env.KUKURI_PEER_PUBLISH_PREFIX?.trim() ||
      DEFAULT_PUBLISH_PREFIX;

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

    const topicButton = await $(`[data-testid="topic-${DEFAULT_PUBLIC_TOPIC_ID}"]`);
    await topicButton.waitForDisplayed({ timeout: 30000 });
    await topicButton.click();
    await waitForTopicRoute(DEFAULT_PUBLIC_TOPIC_ID);

    const realtimeToggle = await $('[data-testid="timeline-mode-toggle-realtime"]');
    await realtimeToggle.waitForDisplayed({ timeout: 15000 });
    await realtimeToggle.click();
    await browser.waitUntil(
      async () => (await getTimelineUpdateMode()).mode === 'realtime',
      {
        timeout: 15000,
        interval: 500,
        timeoutMsg: 'Timeline update mode did not switch to realtime',
      },
    );

    const joinButton = await $('[data-testid="topic-mesh-join-button"]');
    if (await joinButton.isExisting()) {
      await joinButton.waitForDisplayed({ timeout: 20000 });
      await joinButton.click();
    }

    await waitForActiveTopic(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForBackendPeerConnectivity(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForTopicMeshPeerCount();
    const modeBeforePropagation = await getTimelineUpdateMode();

    const baselineTimelineSnapshot = await getTimelineThreadSnapshot(publishPrefix);
    const baselineP2PSnapshot = await getP2PMessageSnapshot(DEFAULT_PUBLIC_TOPIC_ID);
    const latestP2PSnapshot = await waitForP2PMessageAdvance(
      DEFAULT_PUBLIC_TOPIC_ID,
      publishPrefix,
      baselineP2PSnapshot,
      180000,
    );
    const propagatedMarker =
      pickNewP2PContentMarker(baselineP2PSnapshot, latestP2PSnapshot, publishPrefix) ??
      publishPrefix;

    await waitForRecentMeshMessage(propagatedMarker);
    const topicTimelineQueryBeforeRefresh =
      await getTopicTimelineQuerySnapshot(DEFAULT_PUBLIC_TOPIC_ID);

    const withoutReloadTimelineResult = await waitForTimelineSnapshotChange(
      baselineTimelineSnapshot,
      propagatedMarker,
      15000,
    );
    const renderedWithoutReload = await waitForBodyText(propagatedMarker, 15000);

    await browser.refresh();
    await waitForAppReady();
    await waitForTopicRoute(DEFAULT_PUBLIC_TOPIC_ID);

    const afterReloadTimelineSnapshot = await getTimelineThreadSnapshot(propagatedMarker);
    const renderedAfterReload = await waitForBodyText(propagatedMarker, 60000);
    const postStoreAfterPropagation = await getPostStoreSnapshot(DEFAULT_PUBLIC_TOPIC_ID);

    expect(afterReloadTimelineSnapshot.containsNeedle).toBe(true);
    expect(renderedAfterReload).toBe(true);
    expect(
      postStoreAfterPropagation.recentContents.some((content) => content.includes(propagatedMarker)),
    ).toBe(true);

    if (!withoutReloadTimelineResult.changed) {
      throw new Error(
        `Realtime timeline did not update before refresh; modeBeforePropagation=${modeBeforePropagation.mode}; timelineQueryCountBeforeRefresh=${topicTimelineQueryBeforeRefresh.count}; timelineQueryStatus=${topicTimelineQueryBeforeRefresh.status}; timelineQueryFetchStatus=${topicTimelineQueryBeforeRefresh.fetchStatus}; timelineQueryParents=${topicTimelineQueryBeforeRefresh.parentContents.join(' | ')}; propagatedMarker=${propagatedMarker}; afterReloadContains=${afterReloadTimelineSnapshot.containsNeedle}; beforeRefreshDigest=${withoutReloadTimelineResult.snapshot.textDigest}`,
      );
    }
    if (!withoutReloadTimelineResult.snapshot.containsNeedle) {
      throw new Error(
        `Realtime timeline changed but propagated post was still missing before refresh; propagatedMarker=${propagatedMarker}`,
      );
    }
    if (!renderedWithoutReload) {
      throw new Error(
        `Realtime topic page body did not render propagated post before refresh; propagatedMarker=${propagatedMarker}`,
      );
    }
  });
});
