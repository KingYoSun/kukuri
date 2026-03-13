import { $, browser, expect } from '@wdio/globals';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  findTopicContent,
  getP2PStatus,
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
} from '../helpers/peerHarness';

const DEFAULT_PUBLIC_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_OUTPUT_GROUP = 'community-node-e2e';
const DEFAULT_REPLY_PEER = 'peer-client-2';
const HEX_64_PATTERN = /^[0-9a-f]{64}$/i;

const profile: ProfileInfo = {
  name: 'E2E Thread Preview',
  displayName: 'community-node-thread-preview',
  about: 'Community Node thread preview/detail regression coverage',
};

interface PublishSummary {
  command_id?: string;
  event_id?: string | null;
  published_count?: number;
  processed_at?: string;
  topic_id?: string | null;
  content?: string | null;
}

interface TopicContentState {
  p2pCount: number;
  p2pMessageIds: string[];
  p2pContents: string[];
  postCount: number;
  postIds: string[];
  postEventIds: Array<string | null>;
  postContents: string[];
}

const runPeerHarnessPublish = async (options: {
  peerName: string;
  outputGroup: string;
  topicId: string;
  content: string;
  replyTo?: string;
}): Promise<PublishSummary> => {
  const { commandId } = enqueuePeerHarnessPublishCommand({
    peerName: options.peerName,
    outputGroup: options.outputGroup,
    topicId: options.topicId,
    content: options.content,
    replyToEventId: options.replyTo,
  });

  const result = await waitForPeerHarnessCommandResult({
    peerName: options.peerName,
    outputGroup: options.outputGroup,
    commandId,
    timeoutMs: 120000,
    description: `${options.peerName} should publish ${options.content}`,
  });

  return {
    command_id: result.command_id,
    event_id: result.event_id,
    published_count: result.published_count,
    processed_at: result.processed_at,
    topic_id: result.topic_id,
    content: result.content,
  };
};

const waitForTopicContent = async (
  topicId: string,
  needle: string,
): Promise<TopicContentState> => {
  let latest = await findTopicContent(topicId, needle);

  await browser.waitUntil(
    async () => {
      latest = await findTopicContent(topicId, needle);
      return latest.p2pCount > 0 || latest.postCount > 0;
    },
    {
      timeout: 60000,
      interval: 1000,
      timeoutMsg: `Thread content did not reach topic stores: ${needle}; latest=${JSON.stringify(
        latest,
      )}`,
    },
  );

  return latest;
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

describe('Community Node thread preview/detail UX', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('updates thread preview with propagated replies and opens full thread detail from the preview', async function () {
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
    const replyPeerName = process.env.E2E_COMMUNITY_NODE_REPLY_PEER?.trim() || DEFAULT_REPLY_PEER;

    const rootContent = `community-node-thread-root-${Date.now()}`;
    const replyContent = `community-node-thread-reply-${Date.now()}`;

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

    const joinButton = await $('[data-testid="topic-mesh-join-button"]');
    if (await joinButton.isExisting()) {
      await joinButton.waitForDisplayed({ timeout: 20000 });
      await joinButton.click();
    }

    await waitForActiveTopic(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForBackendPeerConnectivity(DEFAULT_PUBLIC_TOPIC_ID);
    await waitForTopicMeshPeerCount();

    await $('[data-testid="create-post-button"]').click();
    const postInput = await $('[data-testid="post-input"]');
    await postInput.waitForDisplayed({ timeout: 20000 });
    await postInput.setValue(rootContent);
    await $('[data-testid="submit-post-button"]').click();

    let resolvedThreadUuid: string | null = null;
    await browser.waitUntil(
      async () => {
        const cards = await $$('[data-testid^="timeline-thread-card-"]');
        for (const card of cards) {
          if (!(await card.isExisting())) {
            continue;
          }
          const text = await card.getText();
          if (!text.includes(rootContent)) {
            continue;
          }
          const testId = await card.getAttribute('data-testid');
          if (!testId) {
            continue;
          }
          resolvedThreadUuid = testId.replace('timeline-thread-card-', '');
          return resolvedThreadUuid.length > 0;
        }
        return false;
      },
      {
        timeout: 60000,
        interval: 500,
        timeoutMsg: `Locally created thread root did not render in timeline: ${rootContent}`,
      },
    );
    if (!resolvedThreadUuid) {
      throw new Error(`Thread UUID could not be resolved from timeline card: ${rootContent}`);
    }

    let rootEventId: string | null = null;
    await browser.waitUntil(
      async () => {
        const matches = await findTopicContent(DEFAULT_PUBLIC_TOPIC_ID, rootContent);
        if (matches.postCount === 0) {
          return false;
        }
        const candidate = matches.postEventIds.find(
          (value): value is string => typeof value === 'string' && HEX_64_PATTERN.test(value),
        );
        if (!candidate || !HEX_64_PATTERN.test(candidate)) {
          return false;
        }
        rootEventId = candidate;
        return true;
      },
      {
        timeout: 60000,
        interval: 500,
        timeoutMsg: `Locally created root did not resolve to persisted event id: ${rootContent}`,
      },
    );
    if (!rootEventId) {
      throw new Error(`Root event id could not be resolved from post store: ${rootContent}`);
    }

    const parentCard = await $(`[data-testid="timeline-thread-parent-${resolvedThreadUuid}"]`);
    await parentCard.waitForClickable({ timeout: 20000 });
    await parentCard.click();

    const previewPane = await $('[data-testid="thread-preview-pane"]');
    await previewPane.waitForDisplayed({ timeout: 20000 });
    await browser.waitUntil(
      async () => (await previewPane.getText()).includes(rootContent),
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: `Thread preview did not render root content: ${rootContent}`,
      },
    );

    let replySnapshot: TopicContentState | null = null;
    let lastReplySummary: PublishSummary | null = null;
    for (let attempt = 1; attempt <= 2; attempt += 1) {
      lastReplySummary = await runPeerHarnessPublish({
        peerName: replyPeerName,
        outputGroup,
        topicId: DEFAULT_PUBLIC_TOPIC_ID,
        content: replyContent,
        replyTo: rootEventId,
      });

      try {
        replySnapshot = await waitForTopicContent(DEFAULT_PUBLIC_TOPIC_ID, replyContent);
        break;
      } catch (error) {
        if (attempt === 2) {
          throw new Error(
            `${error instanceof Error ? error.message : String(error)}; lastSummary=${JSON.stringify(
              lastReplySummary,
            )}`,
            { cause: error },
          );
        }
      }
    }
    if (!replySnapshot) {
      throw new Error(
        `Reply publish did not reach snapshots: ${replyContent}; lastSummary=${JSON.stringify(
          lastReplySummary,
        )}`,
      );
    }

    await browser.waitUntil(
      async () => {
        const target = await $(`[data-testid="timeline-thread-first-reply-${resolvedThreadUuid}"]`);
        if (!(await target.isExisting())) {
          return false;
        }
        return (await target.getText()).includes(replyContent);
      },
      {
        timeout: 180000,
        interval: 1000,
        timeoutMsg: `Timeline preview reply did not update in realtime: ${replyContent}; replySnapshot=${JSON.stringify(
          replySnapshot,
        )}`,
      },
    );

    await browser.waitUntil(
      async () => (await previewPane.getText()).includes(replyContent),
      {
        timeout: 180000,
        interval: 1000,
        timeoutMsg: `Thread preview pane did not update in realtime: ${replyContent}`,
      },
    );

    const openFullButton = await $('[data-testid="thread-preview-open-full"]');
    await openFullButton.waitForClickable({ timeout: 20000 });
    await openFullButton.click();

    await browser.waitUntil(
      async () =>
        decodeURIComponent(await browser.getUrl()).includes(
          `/topics/${DEFAULT_PUBLIC_TOPIC_ID}/threads/${resolvedThreadUuid}`,
        ),
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: `Full thread route did not open for ${resolvedThreadUuid}`,
      },
    );

    const threadDetailTitle = await $('[data-testid="thread-detail-title"]');
    await threadDetailTitle.waitForDisplayed({ timeout: 20000 });

    const bodyText = await $('body').getText();
    expect(bodyText).toContain(rootContent);
    expect(bodyText).toContain(replyContent);
    await expect($('[data-testid="thread-list-title"]')).not.toBeDisplayed();
  });
});
