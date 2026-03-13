import { $, browser, expect } from '@wdio/globals';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  ensureTestTopic,
  getBootstrapSnapshot,
  getP2PStatus,
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
const DEFAULT_TOPIC_NAME = 'ipv6-relay-fallback-topic';

const profile: ProfileInfo = {
  name: 'E2E IPv6 Relay',
  displayName: 'ipv6-relay-fallback',
  about: 'IPv6 fallback via relay validation',
};

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

describe('Community node IPv6 relay fallback', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('establishes peer connectivity through bootstrap relay hints under IPv6-priority conditions', async function () {
    this.timeout(420000);

    if (process.env.SCENARIO !== 'multi-peer-e2e') {
      this.skip();
      return;
    }

    const topicId = resolveEnvString(process.env.KUKURI_PEER_TOPIC, DEFAULT_TOPIC_ID);

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await ensureTestTopic({
      name: DEFAULT_TOPIC_NAME,
      topicId,
    });

    let snapshot = await getBootstrapSnapshot();
    await browser.waitUntil(
      async () => {
        snapshot = await getBootstrapSnapshot();
        return (
          snapshot.effectiveNodes.length > 0 &&
          snapshot.effectiveNodes.some((entry) => entry.includes('|relay='))
        );
      },
      {
        timeout: 60000,
        interval: 1000,
        timeoutMsg: 'Relay hint candidates were not available in bootstrap snapshot',
      },
    );

    await joinP2PTopic(topicId, []);
    await setTimelineUpdateMode('realtime');

    await browser.waitUntil(
      async () => {
        const status = await getP2PStatus();
        const activeTopic = status.active_topics.find((topic) => topic.topic_id === topicId);
        return !!activeTopic && activeTopic.peer_count >= 1;
      },
      {
        timeout: 180000,
        interval: 1000,
        timeoutMsg: 'IPv6 relay fallback path did not reach active peer connectivity',
      },
    );

    const finalStatus = await getP2PStatus();
    const finalTopic = finalStatus.active_topics.find((topic) => topic.topic_id === topicId);
    expect(finalTopic?.peer_count ?? 0).toBeGreaterThanOrEqual(1);
  });
});
