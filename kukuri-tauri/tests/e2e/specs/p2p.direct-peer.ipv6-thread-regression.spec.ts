import { $, browser, expect } from '@wdio/globals';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import {
  connectToP2PPeer,
  ensureTestTopic,
  findTopicContent,
  getBootstrapSnapshot,
  getP2PMessageSnapshot,
  getP2PStatus,
  getPostStoreSnapshot,
  joinP2PTopic,
  resetAppState,
  setTimelineUpdateMode,
} from '../helpers/bridge';
import {
  enqueuePeerHarnessPublishCommand,
  waitForPeerHarnessAddressSnapshot,
  waitForPeerHarnessCommandResult,
} from '../helpers/peerHarness';

const DEFAULT_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_TOPIC_NAME = 'direct-peer-ipv6-thread-regression-topic';
const DEFAULT_BOOTSTRAP_PEER =
  '03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233';
const DEFAULT_PUBLISH_PREFIX = 'multi-peer-publisher';
const DEFAULT_DIRECT_PEER_NAME = 'peer-client-2';
const DEFAULT_OUTPUT_GROUP = 'multi-peer-e2e';
const DEFAULT_TIMEOUT_MS = 120000;
const HEX_64_PATTERN = /^[0-9a-f]{64}$/i;

interface PeerAddressSnapshot {
  peer_name?: string;
  node_addresses?: string[];
  relay_urls?: string[];
  connection_hints?: string[];
  preferred_address?: string;
}

interface ParsedPeerHint {
  nodeId: string | null;
  endpoint: string | null;
  hasRelay: boolean;
}

interface TimelineThreadSnapshot {
  ids: string[];
  count: number;
  textDigest: string;
}

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

const profile: ProfileInfo = {
  name: 'E2E Direct Peer IPv6',
  displayName: 'direct-peer-ipv6-thread-regression',
  about: 'Direct peer relay-only timeline/thread regression coverage',
};

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

const parseBootstrapPeers = (): string[] => {
  const peers = (process.env.KUKURI_BOOTSTRAP_PEERS ?? DEFAULT_BOOTSTRAP_PEER)
    .split(',')
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return peers.length > 0 ? peers : [DEFAULT_BOOTSTRAP_PEER];
};

const parsePeerHint = (address: string): ParsedPeerHint => {
  const segments = address
    .split('|')
    .map((segment) => segment.trim())
    .filter((segment) => segment.length > 0);
  if (segments.length === 0) {
    return { nodeId: null, endpoint: null, hasRelay: false };
  }

  let nodeId: string | null = null;
  let endpoint: string | null = null;
  let hasRelay = false;

  const first = segments[0] ?? '';
  if (!first.includes('=')) {
    if (first.includes('@')) {
      const [rawNodeId, rawEndpoint] = first.split('@');
      nodeId = rawNodeId?.trim() || null;
      endpoint = rawEndpoint?.trim() || null;
    } else {
      nodeId = first.trim() || null;
    }
  }

  for (const segment of segments) {
    const separatorIndex = segment.indexOf('=');
    if (separatorIndex <= 0) {
      continue;
    }

    const key = segment.slice(0, separatorIndex).trim().toLowerCase();
    const value = segment.slice(separatorIndex + 1).trim();
    if (!value) {
      continue;
    }

    if (key === 'relay' || key === 'relay_url') {
      hasRelay = true;
      continue;
    }
    if ((key === 'addr' || key === 'ip') && !endpoint) {
      endpoint = value;
      continue;
    }
    if ((key === 'node' || key === 'node_id') && !nodeId) {
      nodeId = value;
    }
  }

  return { nodeId, endpoint, hasRelay };
};

const parsePeerAddressNodeId = (address: string): string | null => parsePeerHint(address).nodeId;

const parsePeerAddressHost = (address: string): string | null => {
  const { endpoint } = parsePeerHint(address);
  if (!endpoint) {
    return null;
  }
  if (endpoint.startsWith('[')) {
    const closingIndex = endpoint.indexOf(']');
    return closingIndex > 1 ? endpoint.slice(1, closingIndex).trim() || null : null;
  }
  const separatorIndex = endpoint.lastIndexOf(':');
  return separatorIndex > 0 ? endpoint.slice(0, separatorIndex).trim() || null : null;
};

const hasRelayHint = (address: string): boolean => parsePeerHint(address).hasRelay;

const pickPeerAddress = (snapshot: PeerAddressSnapshot): string | null => {
  const candidates = [
    ...(snapshot.connection_hints ?? []).map((entry) => entry.trim()),
    snapshot.preferred_address?.trim(),
    ...(snapshot.node_addresses ?? []).map((entry) => entry.trim()),
  ].filter((entry): entry is string => Boolean(entry));
  const dedupedCandidates = Array.from(new Set(candidates));
  const relayCandidate = dedupedCandidates.find((candidate) => hasRelayHint(candidate));
  return relayCandidate ?? dedupedCandidates[0] ?? null;
};

const normalizeNodeId = (value: string): string | null =>
  parsePeerAddressNodeId(value)?.trim().toLowerCase() ?? null;

const hasExpectedBootstrapNode = (effectiveNodes: string[], expectedNodes: string[]): boolean => {
  const expectedNodeIds = expectedNodes
    .map((entry) => normalizeNodeId(entry))
    .filter((entry): entry is string => Boolean(entry));
  if (expectedNodeIds.length === 0) {
    return expectedNodes.some((entry) => effectiveNodes.includes(entry));
  }
  const expectedSet = new Set(expectedNodeIds);
  return effectiveNodes.some((entry) => {
    const nodeId = normalizeNodeId(entry);
    return nodeId ? expectedSet.has(nodeId) : false;
  });
};

const openTopic = async (topicId: string): Promise<void> => {
  const topicButton = await $(`[data-testid="topic-${topicId}"]`);
  await topicButton.waitForDisplayed({ timeout: 30000 });
  await topicButton.click();

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

const waitForBodyText = async (needle: string, timeoutMs: number): Promise<boolean> => {
  try {
    await browser.waitUntil(
      async () =>
        await browser.execute(
          (value: string) => (document.body?.innerText ?? '').includes(value),
          needle,
        ),
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

const getTimelineThreadSnapshot = async (): Promise<TimelineThreadSnapshot> => {
  return await browser.execute(() => {
    const cards = Array.from(
      document.querySelectorAll<HTMLElement>('[data-testid^="timeline-thread-card-"]'),
    );
    return {
      ids: cards
        .map((card) => card.getAttribute('data-testid') ?? '')
        .filter((value) => value.length > 0),
      count: cards.length,
      textDigest: cards.map((card) => card.innerText ?? '').join('\n----\n'),
    };
  });
};

const hasTimelineSnapshotChanged = (
  baseline: TimelineThreadSnapshot,
  current: TimelineThreadSnapshot,
): boolean => {
  if (baseline.count !== current.count || baseline.textDigest !== current.textDigest) {
    return true;
  }
  if (baseline.ids.length !== current.ids.length) {
    return true;
  }
  return baseline.ids.some((id, index) => id !== current.ids[index]);
};

const waitForP2PMessageAdvance = async (
  topicId: string,
  publishPrefix: string,
  baselineCount: number,
): Promise<void> => {
  await browser.waitUntil(
    async () => {
      const latest = await getP2PMessageSnapshot(topicId);
      return (
        latest.count > baselineCount &&
        latest.recentContents.some((content) => content.includes(publishPrefix))
      );
    },
    {
      timeout: DEFAULT_TIMEOUT_MS,
      interval: 1000,
      timeoutMsg: `P2P snapshot did not advance for prefix=${publishPrefix}`,
    },
  );
};

const waitForTopicContent = async (topicId: string, needle: string): Promise<TopicContentState> => {
  let latest = await findTopicContent(topicId, needle);

  await browser.waitUntil(
    async () => {
      latest = await findTopicContent(topicId, needle);
      return latest.p2pCount > 0 || latest.postCount > 0;
    },
    {
      timeout: 60000,
      interval: 1000,
      timeoutMsg: `Topic content did not reach topic stores: ${needle}; latest=${JSON.stringify(
        latest,
      )}`,
    },
  );

  return latest;
};

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
    timeoutMs: DEFAULT_TIMEOUT_MS,
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

const getTopicPeerCount = async (topicId: string): Promise<number> => {
  const status = await getP2PStatus();
  return status.active_topics.find((topic) => topic.topic_id === topicId)?.peer_count ?? 0;
};

const waitForDirectPeerAddress = async (peerName: string, outputGroup: string): Promise<string> => {
  const snapshot = await waitForPeerHarnessAddressSnapshot({
    peerName,
    outputGroup,
    timeoutMs: DEFAULT_TIMEOUT_MS,
    description: `${peerName} address snapshot should be ready`,
    predicate: (candidate) => Boolean(pickPeerAddress(candidate as PeerAddressSnapshot)),
  });
  const peerAddress = pickPeerAddress(snapshot as PeerAddressSnapshot);
  if (!peerAddress) {
    throw new Error(`Peer address could not be resolved for ${peerName}`);
  }
  return peerAddress;
};

const setupRelayOnlyTopic = async (topicId: string, peerAddress: string): Promise<void> => {
  await waitForWelcome();
  await $('[data-testid="welcome-create-account"]').click();
  await completeProfileSetup(profile);
  await waitForHome();

  await ensureTestTopic({
    name: DEFAULT_TOPIC_NAME,
    topicId,
  });

  await joinP2PTopic(topicId, [peerAddress]);
  await setTimelineUpdateMode('realtime');
  await openTopic(topicId);
  await connectToP2PPeer(peerAddress);

  const targetPeerNodeId = parsePeerAddressNodeId(peerAddress);
  await browser.waitUntil(
    async () => {
      const status = await getP2PStatus();
      const peerCount =
        status.active_topics.find((topic) => topic.topic_id === topicId)?.peer_count ?? 0;
      if (peerCount < 1) {
        return false;
      }
      if (!targetPeerNodeId) {
        return true;
      }
      return status.peers.some(
        (peer) =>
          peer.node_id === targetPeerNodeId ||
          peer.address.trim().startsWith(`${targetPeerNodeId}@`),
      );
    },
    {
      timeout: DEFAULT_TIMEOUT_MS,
      interval: 1000,
      timeoutMsg: `Direct peer connection did not become active for ${peerAddress}`,
    },
  );
};

const writeDiagnostics = (payload: Record<string, unknown>, fileName: string) => {
  const diagnosticsPath = resolve(process.cwd(), '..', 'test-results', 'multi-peer-e2e', fileName);
  mkdirSync(dirname(diagnosticsPath), { recursive: true });
  writeFileSync(diagnosticsPath, JSON.stringify(payload, null, 2), 'utf-8');
};

describe('Direct peer relay-only IPv6 regressions', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('renders propagated posts without reload under relay-only direct-peer conditions', async function () {
    this.timeout(420000);

    if (process.env.SCENARIO !== 'multi-peer-e2e') {
      this.skip();
      return;
    }

    const topicId = resolveEnvString(process.env.KUKURI_PEER_TOPIC, DEFAULT_TOPIC_ID);
    const publishPrefix = resolveEnvString(
      process.env.E2E_MULTI_PEER_PUBLISH_PREFIX,
      DEFAULT_PUBLISH_PREFIX,
    );
    const outputGroup = resolveEnvString(
      process.env.KUKURI_PEER_OUTPUT_GROUP,
      DEFAULT_OUTPUT_GROUP,
    );
    const peerName = resolveEnvString(process.env.E2E_DIRECT_PEER_NAME, DEFAULT_DIRECT_PEER_NAME);
    const bootstrapPeers = parseBootstrapPeers();

    let bootstrapSnapshot = await getBootstrapSnapshot();
    if (bootstrapPeers.length > 0) {
      await browser.waitUntil(
        async () => {
          bootstrapSnapshot = await getBootstrapSnapshot();
          return (
            bootstrapSnapshot.effectiveNodes.length > 0 &&
            hasExpectedBootstrapNode(bootstrapSnapshot.effectiveNodes, bootstrapPeers)
          );
        },
        {
          timeout: 60000,
          interval: 1000,
          timeoutMsg: `Effective bootstrap nodes were not ready: expected=${JSON.stringify(
            bootstrapPeers,
          )}`,
        },
      );
    }

    const peerAddress = await waitForDirectPeerAddress(peerName, outputGroup);
    expect(hasRelayHint(peerAddress)).toBe(true);

    await setupRelayOnlyTopic(topicId, peerAddress);

    const baselineTimelineSnapshot = await getTimelineThreadSnapshot();
    const baselineP2PSnapshot = await getP2PMessageSnapshot(topicId);
    const baselinePostStoreSnapshot = await getPostStoreSnapshot(topicId);

    await waitForP2PMessageAdvance(topicId, publishPrefix, baselineP2PSnapshot.count);

    const renderedWithoutReload = await waitForBodyText(publishPrefix, 15000);
    expect(renderedWithoutReload).toBe(true);

    let latestTimelineSnapshot = await getTimelineThreadSnapshot();
    await browser.waitUntil(
      async () => {
        latestTimelineSnapshot = await getTimelineThreadSnapshot();
        return hasTimelineSnapshotChanged(baselineTimelineSnapshot, latestTimelineSnapshot);
      },
      {
        timeout: 15000,
        interval: 500,
        timeoutMsg: `Timeline snapshot did not advance after relay-only propagation: ${publishPrefix}`,
      },
    );

    const postStoreSnapshot = await getPostStoreSnapshot(topicId);
    expect(postStoreSnapshot.count).toBeGreaterThanOrEqual(baselinePostStoreSnapshot.count);

    writeDiagnostics(
      {
        scenario: process.env.SCENARIO ?? null,
        topicId,
        outputGroup,
        peerName,
        peerAddress,
        selectedPeerHasRelayHint: hasRelayHint(peerAddress),
        selectedPeerHost: parsePeerAddressHost(peerAddress),
        bootstrapSnapshot,
        baselinePeerCount: await getTopicPeerCount(topicId),
        baselineP2PSnapshot,
        baselinePostStoreSnapshot,
        latestTimelineSnapshot,
        postStoreSnapshot,
        renderedWithoutReload,
      },
      'direct-peer-ipv6-stale-render.json',
    );
  });

  it('updates thread preview and full thread detail with propagated replies under relay-only direct-peer conditions', async function () {
    this.timeout(480000);

    if (process.env.SCENARIO !== 'multi-peer-e2e') {
      this.skip();
      return;
    }

    const topicId = resolveEnvString(process.env.KUKURI_PEER_TOPIC, DEFAULT_TOPIC_ID);
    const outputGroup = resolveEnvString(
      process.env.KUKURI_PEER_OUTPUT_GROUP,
      DEFAULT_OUTPUT_GROUP,
    );
    const peerName = resolveEnvString(process.env.E2E_DIRECT_PEER_NAME, DEFAULT_DIRECT_PEER_NAME);
    const bootstrapPeers = parseBootstrapPeers();

    let bootstrapSnapshot = await getBootstrapSnapshot();
    if (bootstrapPeers.length > 0) {
      await browser.waitUntil(
        async () => {
          bootstrapSnapshot = await getBootstrapSnapshot();
          return (
            bootstrapSnapshot.effectiveNodes.length > 0 &&
            hasExpectedBootstrapNode(bootstrapSnapshot.effectiveNodes, bootstrapPeers)
          );
        },
        {
          timeout: 60000,
          interval: 1000,
          timeoutMsg: `Effective bootstrap nodes were not ready: expected=${JSON.stringify(
            bootstrapPeers,
          )}`,
        },
      );
    }

    const peerAddress = await waitForDirectPeerAddress(peerName, outputGroup);
    expect(hasRelayHint(peerAddress)).toBe(true);

    await setupRelayOnlyTopic(topicId, peerAddress);

    const rootContent = `direct-peer-thread-root-${Date.now()}`;
    const replyContent = `direct-peer-thread-reply-${Date.now()}`;

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

    let rootSnapshot: TopicContentState | null = null;
    let rootEventId: string | null = null;
    await browser.waitUntil(
      async () => {
        rootSnapshot = await findTopicContent(topicId, rootContent);
        if (rootSnapshot.postCount === 0) {
          return false;
        }
        const candidate = rootSnapshot.postEventIds.find(
          (value): value is string => typeof value === 'string' && HEX_64_PATTERN.test(value),
        );
        if (!candidate) {
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
    await browser.waitUntil(async () => (await previewPane.getText()).includes(rootContent), {
      timeout: 20000,
      interval: 500,
      timeoutMsg: `Thread preview did not render root content: ${rootContent}`,
    });

    const replySummary = await runPeerHarnessPublish({
      peerName,
      outputGroup,
      topicId,
      content: replyContent,
      replyTo: rootEventId,
    });
    const replySnapshot = await waitForTopicContent(topicId, replyContent);

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

    await browser.waitUntil(async () => (await previewPane.getText()).includes(replyContent), {
      timeout: 180000,
      interval: 1000,
      timeoutMsg: `Thread preview pane did not update in realtime: ${replyContent}`,
    });

    const openFullButton = await $('[data-testid="thread-preview-open-full"]');
    await openFullButton.waitForClickable({ timeout: 20000 });
    await openFullButton.click();

    await browser.waitUntil(
      async () =>
        decodeURIComponent(await browser.getUrl()).includes(
          `/topics/${topicId}/threads/${resolvedThreadUuid}`,
        ),
      {
        timeout: 20000,
        interval: 500,
        timeoutMsg: `Full thread route did not open for ${resolvedThreadUuid}`,
      },
    );

    const threadDetailTitle = await $('[data-testid="thread-detail-title"]');
    await threadDetailTitle.waitForDisplayed({ timeout: 20000 });

    await browser.waitUntil(
      async () => {
        const bodyText = await $('body').getText();
        return bodyText.includes(rootContent) && bodyText.includes(replyContent);
      },
      {
        timeout: 60000,
        interval: 1000,
        timeoutMsg: `Full thread detail did not render root/reply content: root=${rootContent}; reply=${replyContent}`,
      },
    );

    await expect($('[data-testid="thread-list-title"]')).not.toBeDisplayed();

    writeDiagnostics(
      {
        scenario: process.env.SCENARIO ?? null,
        topicId,
        outputGroup,
        peerName,
        peerAddress,
        selectedPeerHasRelayHint: hasRelayHint(peerAddress),
        selectedPeerHost: parsePeerAddressHost(peerAddress),
        bootstrapSnapshot,
        rootContent,
        rootEventId,
        rootSnapshot,
        resolvedThreadUuid,
        replyContent,
        replySummary,
        replySnapshot,
        finalUrl: decodeURIComponent(await browser.getUrl()),
      },
      'direct-peer-ipv6-thread-preview-replies.json',
    );
  });
});
