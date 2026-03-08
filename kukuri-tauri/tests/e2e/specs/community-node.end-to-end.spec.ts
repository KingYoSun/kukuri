import { $, $$, browser, expect } from '@wdio/globals';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  communityNodeListBootstrapNodes,
  communityNodeListBootstrapServices,
  getBootstrapSnapshot,
  getP2PNodeAddresses,
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
  waitForPeerHarnessAddressSnapshot,
  waitForPeerHarnessSummary,
} from '../helpers/peerHarness';

const DEFAULT_PUBLIC_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_PUBLISH_PREFIX = 'community-node-peer-publisher';
const DEFAULT_OUTPUT_GROUP = 'community-node-e2e';
const DEFAULT_LISTENER_PEER = 'peer-client-1';

const profile: ProfileInfo = {
  name: 'E2E Community UX',
  displayName: 'community-node-end-to-end',
  about: 'Community Node end-to-end UX validation',
};

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

const extractItems = (value: Record<string, unknown> | null | undefined): unknown[] => {
  const items = value?.items;
  return Array.isArray(items) ? items : [];
};

const parseIntAttribute = async (selector: string, attribute = 'data-count'): Promise<number> => {
  const elements = await $$(selector);
  const element = elements[0];
  if (!element) {
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

const normalizeRelayUrl = (value: string): string => value.trim().replace(/\/+$/, '');
const usesDirectAddrHint = (value: string): boolean =>
  value.includes('@') || value.includes('|addr=');

interface CommunityNodeP2PStatusSnapshot {
  status: string;
  node_id: string | null;
  bind_addr: string;
  relay_urls: string[];
  desired_topics: string[];
  node_topics: string[];
  gossip_topics: string[];
  router_ready: boolean;
}

const deriveCommunityNodeP2PStatusUrls = (
  baseUrl: string,
  relayUrl?: string,
): string[] => {
  const candidates: string[] = [];
  const seen = new Set<string>();
  const pushCandidate = (value: string) => {
    if (!seen.has(value)) {
      seen.add(value);
      candidates.push(value);
    }
  };

  pushCandidate(new URL('/v1/p2p/status', baseUrl).toString());

  if (relayUrl) {
    try {
      const relayEndpoint = new URL(relayUrl);
      relayEndpoint.protocol =
        relayEndpoint.protocol === 'wss:' ? 'https:' : relayEndpoint.protocol === 'ws:' ? 'http:' : relayEndpoint.protocol;
      relayEndpoint.pathname = '/v1/p2p/status';
      relayEndpoint.search = '';
      relayEndpoint.hash = '';
      pushCandidate(relayEndpoint.toString());
    } catch {
      // Ignore invalid relay URL; the primary baseUrl candidate is still available.
    }
  }

  return candidates;
};

const fetchCommunityNodeP2PStatus = async (
  baseUrl: string,
  relayUrl?: string,
): Promise<CommunityNodeP2PStatusSnapshot> => {
  let lastError: Error | null = null;
  for (const candidateUrl of deriveCommunityNodeP2PStatusUrls(baseUrl, relayUrl)) {
    try {
      const response = await fetch(candidateUrl, {
        headers: {
          accept: 'application/json',
        },
      });

      if (!response.ok) {
        const body = await response.text();
        lastError = new Error(`community node p2p status ${response.status} from ${candidateUrl}: ${body}`);
        continue;
      }

      return (await response.json()) as CommunityNodeP2PStatusSnapshot;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
    }
  }

  throw lastError ?? new Error('community node p2p status was unavailable');
};

const collectCommunityNodeSnapshot = async (
  baseUrl: string,
  topicId: string,
  relayUrl?: string,
) => {
  const [bootstrap, relay, p2p, communityNodeP2P, bootstrapNodes, bootstrapServices, page] =
    await Promise.all([
      getBootstrapSnapshot(),
      getRelayStatusSnapshot(),
      getP2PStatus(),
      fetchCommunityNodeP2PStatus(baseUrl, relayUrl).catch((error) => ({
        error: error instanceof Error ? error.message : String(error),
      })),
      communityNodeListBootstrapNodes().catch((error) => ({
        error: error instanceof Error ? error.message : String(error),
      })),
      communityNodeListBootstrapServices(topicId).catch((error) => ({
        error: error instanceof Error ? error.message : String(error),
      })),
      browser.execute(() => {
        const peerCount = document.querySelector('[data-testid="topic-mesh-peer-count"]');
        const joinButton = document.querySelector('[data-testid="topic-mesh-join-button"]');
        const meshCard = document.querySelector('[data-testid="topic-mesh-card"]');
        return {
          pathname: window.location.pathname,
          meshCardPresent: Boolean(meshCard),
          joinButtonPresent: Boolean(joinButton),
          peerCountPresent: Boolean(peerCount),
          peerCountText: peerCount?.textContent ?? null,
          bodyText: document.body?.innerText?.slice(0, 2000) ?? '',
        };
      }),
    ]);

  return {
    bootstrap,
    relay,
    p2p,
    communityNodeP2P,
    bootstrapNodes,
    bootstrapServices,
    page,
  };
};

const waitForExpectedRelayConnection = async (expectedRelayUrl: string): Promise<void> => {
  const normalizedExpected = normalizeRelayUrl(expectedRelayUrl);
  const forbiddenFragments = ['api.kukuri.app/relay'];
  let latestSnapshot:
    | {
        relays: Array<{ url: string; status: string }>;
        error: string | null;
        lastFetchedAt: number | null;
      }
    | null = null;

  await browser
    .waitUntil(
      async () => {
        latestSnapshot = await getRelayStatusSnapshot();
        const normalizedRelayEntries = latestSnapshot.relays.map((relay) => ({
          url: normalizeRelayUrl(relay.url),
          status: relay.status,
        }));

        const hasForbiddenRelay = normalizedRelayEntries.some((relay) =>
          forbiddenFragments.some((fragment) => relay.url.includes(fragment)),
        );
        if (hasForbiddenRelay) {
          return false;
        }

        return normalizedRelayEntries.some(
          (relay) =>
            relay.url === normalizedExpected && relay.status.toLowerCase() === 'connected',
        );
      },
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: `Expected Nostr relay did not connect: ${normalizedExpected}`,
      },
    )
    .catch((error) => {
      const snapshotText = latestSnapshot ? JSON.stringify(latestSnapshot) : 'null';
      throw new Error(
        `${error instanceof Error ? error.message : String(error)}; relayStatus=${snapshotText}`,
      );
    });

  if (!latestSnapshot) {
    throw new Error(`Nostr relay status snapshot was unavailable for ${normalizedExpected}`);
  }

  const normalizedRelayEntries = latestSnapshot.relays.map((relay) => ({
    url: normalizeRelayUrl(relay.url),
    status: relay.status,
  }));
  const forbiddenRelay = normalizedRelayEntries.find((relay) =>
    forbiddenFragments.some((fragment) => relay.url.includes(fragment)),
  );
  if (forbiddenRelay) {
    throw new Error(
      `Unexpected fallback relay remained configured: ${JSON.stringify(latestSnapshot)}`,
    );
  }
};

const waitForRelayOnlyNodeAddresses = async (expectedRelayUrl: string): Promise<void> => {
  const normalizedExpected = normalizeRelayUrl(expectedRelayUrl);
  let latestNodeAddresses: string[] = [];

  await browser
    .waitUntil(
      async () => {
        latestNodeAddresses = await getP2PNodeAddresses();
        if (latestNodeAddresses.length === 0) {
          return false;
        }

        return latestNodeAddresses.every(
          (entry) =>
            !usesDirectAddrHint(entry) &&
            normalizeRelayUrl(entry).includes(`relay=${normalizedExpected}`),
        );
      },
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: `Local node addresses did not converge to relay-only hints: ${normalizedExpected}`,
      },
    )
    .catch((error) => {
      throw new Error(
        `${error instanceof Error ? error.message : String(error)}; nodeAddresses=${JSON.stringify(latestNodeAddresses)}`,
      );
    });
};

const waitForRelayOnlyPeerHarness = async (
  peerName: string,
  outputGroup: string,
  expectedRelayUrl: string,
): Promise<void> => {
  const normalizedExpected = normalizeRelayUrl(expectedRelayUrl);
  const snapshot = await waitForPeerHarnessAddressSnapshot({
    peerName,
    outputGroup,
    timeoutMs: 120000,
    description: `${peerName} should advertise relay-only hints`,
    predicate: (candidate) => {
      const relayUrls = candidate.relay_urls ?? [];
      const nodeAddresses = candidate.node_addresses ?? [];
      const connectionHints = candidate.connection_hints ?? [];
      if (!relayUrls.some((value) => normalizeRelayUrl(value) === normalizedExpected)) {
        return false;
      }
      if (nodeAddresses.length === 0 || connectionHints.length === 0) {
        return false;
      }
      return (
        nodeAddresses.every((value) => !usesDirectAddrHint(value)) &&
        connectionHints.every((value) => !usesDirectAddrHint(value))
      );
    },
  });

  expect((snapshot.relay_urls ?? []).map((value) => normalizeRelayUrl(value))).toContain(
    normalizedExpected,
  );
  expect((snapshot.node_addresses ?? []).every((value) => !usesDirectAddrHint(value))).toBe(true);
  expect((snapshot.connection_hints ?? []).every((value) => !usesDirectAddrHint(value))).toBe(
    true,
  );
};

const waitForCommunityNodeBootstrap = async (
  baseUrl: string,
  topicId: string,
  relayUrl?: string,
): Promise<void> => {
  let lastError: string | null = null;
  try {
    await browser.waitUntil(
      async () => {
        try {
          const snapshot = await getBootstrapSnapshot();
          if (snapshot.effectiveNodes.length === 0) {
            return false;
          }

          const bootstrapNodes = await communityNodeListBootstrapNodes();
          if (extractItems(bootstrapNodes).length === 0) {
            return false;
          }

          const bootstrapServices = await communityNodeListBootstrapServices(topicId);
          return extractItems(bootstrapServices).length > 0;
        } catch (error) {
          lastError = error instanceof Error ? error.message : String(error);
          return false;
        }
      },
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: `Community node bootstrap data was not ready for ${topicId}`,
      },
    );
  } catch (error) {
    const snapshot = await collectCommunityNodeSnapshot(baseUrl, topicId, relayUrl);
    const suffix = lastError ? `; lastError=${lastError}` : '';
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}${suffix}; snapshot=${JSON.stringify(snapshot)}`,
      { cause: error },
    );
  }
};

const waitForCommunityNodeTopicSubscription = async (
  baseUrl: string,
  topicId: string,
  relayUrl?: string,
): Promise<void> => {
  try {
    await browser.waitUntil(
      async () => {
        const status = await fetchCommunityNodeP2PStatus(baseUrl, relayUrl);
        return (
          status.router_ready &&
          status.desired_topics.includes(topicId) &&
          status.node_topics.includes(topicId) &&
          status.gossip_topics.includes(topicId)
        );
      },
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: `Community node did not subscribe to gossip topic ${topicId}`,
      },
    );
  } catch (error) {
    const snapshot = await collectCommunityNodeSnapshot(baseUrl, topicId, relayUrl);
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}; snapshot=${JSON.stringify(snapshot)}`,
      { cause: error },
    );
  }
};

const waitForActiveTopic = async (
  baseUrl: string,
  topicId: string,
  relayUrl?: string,
): Promise<void> => {
  try {
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
  } catch (error) {
    const snapshot = await collectCommunityNodeSnapshot(baseUrl, topicId, relayUrl);
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}; snapshot=${JSON.stringify(snapshot)}`,
      { cause: error },
    );
  }
};

const waitForBackendPeerConnectivity = async (
  baseUrl: string,
  topicId: string,
  relayUrl?: string,
): Promise<void> => {
  try {
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
  } catch (error) {
    const snapshot = await collectCommunityNodeSnapshot(baseUrl, topicId, relayUrl);
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}; snapshot=${JSON.stringify(snapshot)}`,
      { cause: error },
    );
  }
};

const waitForTopicMeshPeerCount = async (
  baseUrl: string,
  topicId: string,
  relayUrl?: string,
): Promise<void> => {
  try {
    await browser.waitUntil(
      async () => (await parseIntAttribute('[data-testid="topic-mesh-peer-count"]')) > 0,
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: 'Topic mesh peer count did not become positive',
      },
    );
  } catch (error) {
    const snapshot = await collectCommunityNodeSnapshot(baseUrl, topicId, relayUrl);
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}; snapshot=${JSON.stringify(snapshot)}`,
      { cause: error },
    );
  }
};

const waitForRecentMeshMessage = async (needle: string): Promise<void> => {
  await browser.waitUntil(
    async () => {
      const messages = await $$('[data-testid="topic-mesh-recent-message"]');
      for (const message of messages) {
        const text = await message.getText();
        if (text.includes(needle)) {
          return true;
        }
      }
      return false;
    },
    {
      timeout: 180000,
      interval: 1000,
      timeoutMsg: `Topic mesh did not receive expected message: ${needle}`,
    },
  );
};

describe('Community Node end-to-end UX', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('adds/authenticates community node, joins topic mesh, and propagates posts end-to-end', async function () {
    this.timeout(480000);

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
    const expectedIrohRelayUrl = resolveEnvString(
      process.env.KUKURI_IROH_RELAY_URLS,
      'http://cn-iroh-relay:3340',
    );

    const publishPrefix = resolveEnvString(
      process.env.E2E_MULTI_PEER_PUBLISH_PREFIX ?? process.env.KUKURI_PEER_PUBLISH_PREFIX,
      DEFAULT_PUBLISH_PREFIX,
    );
    const listenerPeer = resolveEnvString(
      process.env.E2E_COMMUNITY_NODE_LISTENER_PEER,
      DEFAULT_LISTENER_PEER,
    );
    const outputGroup = resolveEnvString(process.env.KUKURI_PEER_OUTPUT_GROUP, DEFAULT_OUTPUT_GROUP);

    await waitForWelcome();
    await startCreateAccountFlow();
    await completeProfileSetup(profile);
    await waitForHome();

    await runCommunityNodeAuthFlow(baseUrl);
    await waitForExpectedRelayConnection(expectedRelayUrl);
    await waitForCommunityNodeBootstrap(baseUrl, DEFAULT_PUBLIC_TOPIC_ID, expectedRelayUrl);
    await waitForCommunityNodeTopicSubscription(baseUrl, DEFAULT_PUBLIC_TOPIC_ID, expectedRelayUrl);
    await waitForRelayOnlyNodeAddresses(expectedIrohRelayUrl);
    await waitForRelayOnlyPeerHarness('peer-client-1', outputGroup, expectedIrohRelayUrl);
    await waitForRelayOnlyPeerHarness('peer-client-2', outputGroup, expectedIrohRelayUrl);

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

    await waitForActiveTopic(baseUrl, DEFAULT_PUBLIC_TOPIC_ID, expectedRelayUrl);
    await waitForBackendPeerConnectivity(baseUrl, DEFAULT_PUBLIC_TOPIC_ID, expectedRelayUrl);
    await waitForTopicMeshPeerCount(baseUrl, DEFAULT_PUBLIC_TOPIC_ID, expectedRelayUrl);
    await waitForRecentMeshMessage(publishPrefix);

    await browser.waitUntil(
      async () =>
        await browser.execute((needle: string) => {
          const text = document.body?.innerText ?? '';
          return text.includes(needle);
        }, publishPrefix),
      {
        timeout: 180000,
        interval: 1000,
        timeoutMsg: `Timeline did not render propagated publisher message: ${publishPrefix}`,
      },
    );

    const uniquePost = `community-node-e2e-post-${Date.now()}`;
    await $('[data-testid="create-post-button"]').click();
    const postInput = await $('[data-testid="post-input"]');
    await postInput.waitForDisplayed({ timeout: 15000 });
    await postInput.setValue(uniquePost);
    await $('[data-testid="submit-post-button"]').click();

    const listenerSummary = await waitForPeerHarnessSummary({
      peerName: listenerPeer,
      outputGroup,
      timeoutMs: 180000,
      description: `listener peer should receive propagated post: ${uniquePost}`,
      predicate: (summary) =>
        (summary.stats?.received_count ?? 0) > 0 &&
        (summary.stats?.recent_contents ?? []).some((content) => content.includes(uniquePost)),
    });

    expect(
      (listenerSummary.stats?.recent_contents ?? []).some((content) => content.includes(uniquePost)),
    ).toBe(true);
  });
});
