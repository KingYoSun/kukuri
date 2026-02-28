import { $, browser, expect } from '@wdio/globals';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  ensureTestTopic,
  getP2PMessageSnapshot,
  getP2PStatus,
  getPostStoreSnapshot,
  joinP2PTopic,
  resetAppState,
  setTimelineUpdateMode,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';

const DEFAULT_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_TOPIC_NAME = 'multi-peer-e2e-topic';
const DEFAULT_BOOTSTRAP_PEER =
  '03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233';
const DEFAULT_PREFIX = 'multi-peer-publisher';

const profile: ProfileInfo = {
  name: 'E2E Multi Peer',
  displayName: 'multi-peer-e2e',
  about: 'Multi-peer propagation validation',
};

const parseBootstrapPeers = (): string[] => {
  const peers = (process.env.KUKURI_BOOTSTRAP_PEERS ?? DEFAULT_BOOTSTRAP_PEER)
    .split(',')
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return peers.length > 0 ? peers : [DEFAULT_BOOTSTRAP_PEER];
};

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

const expectedPeerCount = (): number => {
  const raw = Number(process.env.E2E_MULTI_PEER_EXPECTED_MIN ?? '1');
  if (!Number.isFinite(raw) || raw < 0) {
    return 1;
  }
  return Math.floor(raw);
};

describe('Multi-peer docker propagation', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('observes peer connectivity and propagated posts from docker peer clients', async function () {
    this.timeout(420000);

    if (process.env.SCENARIO !== 'multi-peer-e2e') {
      this.skip();
      return;
    }

    const topicId = resolveEnvString(process.env.KUKURI_PEER_TOPIC, DEFAULT_TOPIC_ID);
    const publishPrefix = resolveEnvString(process.env.E2E_MULTI_PEER_PUBLISH_PREFIX, DEFAULT_PREFIX);
    const bootstrapPeers = parseBootstrapPeers();

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await ensureTestTopic({
      name: DEFAULT_TOPIC_NAME,
      topicId,
    });

    await joinP2PTopic(topicId, bootstrapPeers);
    await setTimelineUpdateMode('realtime');

    const topicButton = await $(`[data-testid="topic-${topicId}"]`);
    await topicButton.waitForDisplayed({ timeout: 30000 });
    await topicButton.click();

    const encodedTopicId = encodeURIComponent(topicId);
    await browser.waitUntil(
      async () => {
        const currentUrl = decodeURIComponent(await browser.getUrl());
        return currentUrl.includes(`/topics/${topicId}`) || currentUrl.includes(`/topics/${encodedTopicId}`);
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: `topic route did not open: ${topicId}`,
      },
    );

    const minPeers = expectedPeerCount();
    await browser.waitUntil(
      async () => {
        const status = await getP2PStatus();
        const activeTopic = status.active_topics.find((topic) => topic.topic_id === topicId);
        return !!activeTopic && activeTopic.peer_count >= minPeers;
      },
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: `P2P topic status did not reach expected peer count (>=${minPeers})`,
      },
    );

    await browser.waitUntil(
      async () => {
        const p2pSnapshot = await getP2PMessageSnapshot(topicId);
        const postStoreSnapshot = await getPostStoreSnapshot(topicId);
        const inP2P = p2pSnapshot.recentContents.some((content) => content.includes(publishPrefix));
        const inStore = postStoreSnapshot.recentContents.some((content) =>
          content.includes(publishPrefix),
        );
        return inP2P || inStore;
      },
      {
        timeout: 180000,
        interval: 1000,
        timeoutMsg: `Did not observe propagated payload prefix: ${publishPrefix}`,
      },
    );

    await browser.waitUntil(
      async () =>
        await browser.execute((needle: string) => {
          const text = document.body?.innerText ?? '';
          return text.includes(needle);
        }, publishPrefix),
      {
        timeout: 60000,
        interval: 1000,
        timeoutMsg: `Topic page did not render payload prefix: ${publishPrefix}`,
      },
    );

    const screenshotPath = resolveEnvString(
      process.env.E2E_MULTI_PEER_SCREENSHOT_PATH,
      resolve(
        process.cwd(),
        '..',
        'test-results',
        'multi-peer-e2e',
        'multi-peer-propagation.png',
      ),
    );
    mkdirSync(dirname(screenshotPath), { recursive: true });
    await browser.saveScreenshot(screenshotPath);

    const finalStatus = await getP2PStatus();
    const finalTopic = finalStatus.active_topics.find((topic) => topic.topic_id === topicId);
    expect(finalTopic?.peer_count ?? 0).toBeGreaterThanOrEqual(minPeers);
  });
});
