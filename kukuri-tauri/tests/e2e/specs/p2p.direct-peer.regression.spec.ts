import { $, browser, expect } from '@wdio/globals';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
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
  getBootstrapSnapshot,
  getP2PMessageSnapshot,
  getP2PStatus,
  getPostStoreSnapshot,
  joinP2PTopic,
  resetAppState,
  setTimelineUpdateMode,
} from '../helpers/bridge';

const DEFAULT_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_TOPIC_NAME = 'direct-peer-regression-topic';
const DEFAULT_BOOTSTRAP_PEER =
  '03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233';
const DEFAULT_PUBLISH_PREFIX = 'multi-peer-publisher';
const DEFAULT_EXPECTED_PROFILE_NAME = 'multi-peer-publisher-profile';
const DEFAULT_DIRECT_PEER_NAME = 'peer-client-2';
const DEFAULT_OUTPUT_GROUP = 'multi-peer-e2e';
const DEFAULT_DIRECT_PEER_TIMEOUT_MS = 120000;

interface PeerAddressSnapshot {
  peer_name?: string;
  node_addresses?: string[];
  relay_urls?: string[];
  connection_hints?: string[];
  preferred_address?: string;
}

interface PeerSummarySnapshot {
  peer_name?: string;
  stats?: {
    published_count?: number;
    metadata_published_count?: number;
    received_count?: number;
  };
}

interface TimelineThreadSnapshot {
  ids: string[];
  count: number;
  textDigest: string;
  containsNeedle: boolean;
}

const profile: ProfileInfo = {
  name: 'E2E Direct Peer',
  displayName: 'direct-peer-regression',
  about: 'Direct peer realtime propagation validation',
};

const resolveEnvString = (value: string | undefined, fallback: string): string => {
  const trimmed = value?.trim();
  return trimmed && trimmed.length > 0 ? trimmed : fallback;
};

const parseOptionalBool = (value: string | undefined): boolean | null => {
  if (value === undefined) {
    return null;
  }
  const normalized = value.trim().toLowerCase();
  if (normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on') {
    return true;
  }
  if (normalized === '0' || normalized === 'false' || normalized === 'no' || normalized === 'off') {
    return false;
  }
  return null;
};

const parseTimeoutMs = (value: string | undefined, fallbackMs: number): number => {
  const trimmed = value?.trim();
  if (!trimmed) {
    return fallbackMs;
  }
  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return fallbackMs;
  }
  return Math.floor(parsed);
};

const parseOptionalInt = (value: string | undefined): number | null => {
  const trimmed = value?.trim();
  if (!trimmed) {
    return null;
  }
  const parsed = Number(trimmed);
  if (!Number.isFinite(parsed)) {
    return null;
  }
  return Math.floor(parsed);
};

type DirectPeerConnectMode = 'bootstrap' | 'direct' | 'both';

const resolveConnectMode = (): DirectPeerConnectMode => {
  const raw = process.env.E2E_DIRECT_PEER_CONNECT_MODE?.trim().toLowerCase();
  if (raw === 'bootstrap' || raw === 'direct' || raw === 'both') {
    return raw;
  }
  return 'both';
};

const shouldUseDirectJoinHints = (mode: DirectPeerConnectMode): boolean =>
  mode === 'direct' || mode === 'both';

const shouldCallDirectConnect = (mode: DirectPeerConnectMode): boolean => {
  const explicit = parseOptionalBool(process.env.E2E_DIRECT_PEER_CALL_CONNECT);
  if (explicit !== null) {
    return explicit;
  }
  return mode !== 'bootstrap';
};

const parseBootstrapPeers = (): string[] => {
  const useBootstrap =
    parseOptionalBool(process.env.E2E_DIRECT_PEER_USE_BOOTSTRAP) ??
    process.env.SCENARIO === 'multi-peer-e2e';
  if (!useBootstrap) {
    return [];
  }
  const peers = (process.env.KUKURI_BOOTSTRAP_PEERS ?? DEFAULT_BOOTSTRAP_PEER)
    .split(',')
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
  return peers.length > 0 ? peers : [DEFAULT_BOOTSTRAP_PEER];
};

const normalizeNodeId = (value: string): string | null =>
  parsePeerAddressNodeId(value)?.trim() ?? null;

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

const addressSnapshotCandidates = (): string[] => {
  const outputGroup = resolveEnvString(process.env.KUKURI_PEER_OUTPUT_GROUP, DEFAULT_OUTPUT_GROUP);
  const peerName = resolveEnvString(process.env.E2E_DIRECT_PEER_NAME, DEFAULT_DIRECT_PEER_NAME);
  const fileName = `${peerName}.address.json`;
  return [
    resolve('/app/test-results', outputGroup, fileName),
    resolve(process.cwd(), '..', 'test-results', outputGroup, fileName),
  ];
};

const summarySnapshotCandidates = (): string[] => {
  const outputGroup = resolveEnvString(process.env.KUKURI_PEER_OUTPUT_GROUP, DEFAULT_OUTPUT_GROUP);
  const peerName = resolveEnvString(process.env.E2E_DIRECT_PEER_NAME, DEFAULT_DIRECT_PEER_NAME);
  const fileName = `${peerName}.json`;
  return [
    resolve('/app/test-results', outputGroup, fileName),
    resolve(process.cwd(), '..', 'test-results', outputGroup, fileName),
  ];
};

interface ParsedPeerHint {
  nodeId: string | null;
  endpoint: string | null;
  hasRelay: boolean;
}

const parsePeerHint = (address: string): ParsedPeerHint => {
  const segments = address
    .split('|')
    .map((segment) => segment.trim())
    .filter((segment) => segment.length > 0);
  if (segments.length === 0) {
    return {
      nodeId: null,
      endpoint: null,
      hasRelay: false,
    };
  }

  let nodeId: string | null = null;
  let endpoint: string | null = null;
  let hasRelay = false;

  const first = segments[0] ?? '';
  if (!first.includes('=')) {
    if (first.includes('@')) {
      const [rawNodeId, rawEndpoint] = first.split('@');
      const normalizedNodeId = rawNodeId?.trim();
      const normalizedEndpoint = rawEndpoint?.trim();
      nodeId = normalizedNodeId && normalizedNodeId.length > 0 ? normalizedNodeId : null;
      endpoint = normalizedEndpoint && normalizedEndpoint.length > 0 ? normalizedEndpoint : null;
    } else {
      const normalizedNodeId = first.trim();
      nodeId = normalizedNodeId.length > 0 ? normalizedNodeId : null;
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

const parsePeerAddressHost = (address: string): string | null => {
  const { endpoint } = parsePeerHint(address);
  if (!endpoint) {
    return null;
  }
  const trimmedEndpoint = endpoint.trim();
  if (!trimmedEndpoint) {
    return null;
  }
  if (trimmedEndpoint.startsWith('[')) {
    const closingIndex = trimmedEndpoint.indexOf(']');
    if (closingIndex <= 1) {
      return null;
    }
    return trimmedEndpoint.slice(1, closingIndex).trim() || null;
  }
  const separatorIndex = trimmedEndpoint.lastIndexOf(':');
  if (separatorIndex <= 0) {
    return null;
  }
  return trimmedEndpoint.slice(0, separatorIndex).trim() || null;
};

const parsePeerAddressNodeId = (address: string): string | null => {
  return parsePeerHint(address).nodeId;
};

const hasRelayHint = (address: string): boolean => parsePeerHint(address).hasRelay;

const isLoopbackHost = (host: string): boolean => {
  const normalized = host.trim().toLowerCase();
  return normalized === '127.0.0.1' || normalized === 'localhost' || normalized === '::1';
};

const isPrivateIpv4Host = (host: string): boolean => {
  const segments = host.split('.').map((segment) => Number(segment));
  if (segments.length !== 4 || segments.some((segment) => !Number.isInteger(segment))) {
    return false;
  }
  const [a, b] = segments;
  if (a === 10) {
    return true;
  }
  if (a === 172 && b >= 16 && b <= 31) {
    return true;
  }
  return a === 192 && b === 168;
};

const isLocalRoutableHost = (host: string): boolean => {
  if (isLoopbackHost(host)) {
    return true;
  }
  return isPrivateIpv4Host(host);
};

const pickPeerAddress = (snapshot: PeerAddressSnapshot): string | null => {
  const candidates = [
    ...(snapshot.connection_hints ?? []).map((entry) => entry.trim()),
    snapshot.preferred_address?.trim(),
    ...(snapshot.node_addresses ?? []).map((entry) => entry.trim()),
  ].filter((entry): entry is string => Boolean(entry));
  const dedupedCandidates = Array.from(new Set(candidates));

  if (dedupedCandidates.length === 0) {
    return null;
  }

  const preferRelayHint =
    parseOptionalBool(process.env.E2E_DIRECT_PEER_PREFER_RELAY) ??
    process.env.SCENARIO === 'multi-peer-e2e';
  if (preferRelayHint) {
    const relayCandidate = dedupedCandidates.find((candidate) => hasRelayHint(candidate));
    if (relayCandidate) {
      return relayCandidate;
    }
  }

  const preferLocalAddress =
    parseOptionalBool(process.env.E2E_DIRECT_PEER_PREFER_LOCAL_ADDRESS) ??
    process.env.SCENARIO === 'multi-peer-e2e';
  if (preferLocalAddress) {
    const localCandidate = dedupedCandidates.find((candidate) => {
      const host = parsePeerAddressHost(candidate);
      return host ? isLocalRoutableHost(host) : false;
    });
    if (localCandidate) {
      return localCandidate;
    }
  }

  return dedupedCandidates[0] ?? null;
};

const waitForPeerAddress = async (): Promise<string> => {
  const timeoutMs = parseTimeoutMs(
    process.env.E2E_DIRECT_PEER_ADDRESS_TIMEOUT_MS,
    DEFAULT_DIRECT_PEER_TIMEOUT_MS,
  );
  const deadline = Date.now() + timeoutMs;
  let lastError: string | null = null;

  while (Date.now() < deadline) {
    for (const candidate of addressSnapshotCandidates()) {
      if (!existsSync(candidate)) {
        continue;
      }
      try {
        const parsed = JSON.parse(readFileSync(candidate, 'utf-8')) as PeerAddressSnapshot;
        const peerAddress = pickPeerAddress(parsed);
        if (peerAddress) {
          return peerAddress;
        }
        lastError = `Address snapshot exists but no address found (${candidate})`;
      } catch (error) {
        lastError = error instanceof Error ? error.message : String(error);
      }
    }
    await browser.pause(500);
  }

  const candidates = addressSnapshotCandidates().join(', ');
  throw new Error(
    `Timed out waiting for peer address snapshot (candidates: ${candidates}, lastError: ${lastError ?? 'none'})`,
  );
};

const waitForPeerSummary = async (): Promise<PeerSummarySnapshot> => {
  const timeoutMs = parseTimeoutMs(
    process.env.E2E_DIRECT_PEER_SUMMARY_TIMEOUT_MS,
    DEFAULT_DIRECT_PEER_TIMEOUT_MS,
  );
  const deadline = Date.now() + timeoutMs;
  let lastError: string | null = null;

  while (Date.now() < deadline) {
    for (const candidate of summarySnapshotCandidates()) {
      if (!existsSync(candidate)) {
        continue;
      }
      try {
        const parsed = JSON.parse(readFileSync(candidate, 'utf-8')) as PeerSummarySnapshot;
        return parsed;
      } catch (error) {
        lastError = error instanceof Error ? error.message : String(error);
      }
    }
    await browser.pause(500);
  }

  const candidates = summarySnapshotCandidates().join(', ');
  throw new Error(
    `Timed out waiting for peer summary snapshot (candidates: ${candidates}, lastError: ${lastError ?? 'none'})`,
  );
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
      timeoutMsg: `topic route did not open: ${topicId}`,
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

type PostStoreSnapshot = Awaited<ReturnType<typeof getPostStoreSnapshot>>;
type P2PMessageSnapshot = Awaited<ReturnType<typeof getP2PMessageSnapshot>>;
type P2PStatusSnapshot = Awaited<ReturnType<typeof getP2PStatus>>;

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

  const fallback = latest.recentContents.find(
    (content) => content.includes(publishPrefix) && !baselineContents.has(content),
  );
  return fallback ?? null;
};

const getTopicPeerCount = (status: P2PStatusSnapshot, topicId: string): number =>
  status.active_topics.find((topic) => topic.topic_id === topicId)?.peer_count ?? 0;

const hasPeerNodeId = (status: P2PStatusSnapshot, nodeId: string): boolean =>
  status.peers.some(
    (peer) => peer.node_id === nodeId || peer.address.trim().startsWith(`${nodeId}@`),
  );

const isTargetPeerConnected = (
  status: P2PStatusSnapshot,
  topicId: string,
  targetNodeId: string | null,
): boolean => {
  if (targetNodeId) {
    return hasPeerNodeId(status, targetNodeId);
  }
  return getTopicPeerCount(status, topicId) >= 1;
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

const setupTopicPageForDirectPeer = async (
  topicId: string,
  initialPeers: string[] = [],
): Promise<void> => {
  await waitForWelcome();
  await $('[data-testid="welcome-create-account"]').click();
  await completeProfileSetup(profile);
  await waitForHome();

  await ensureTestTopic({
    name: DEFAULT_TOPIC_NAME,
    topicId,
  });

  await joinP2PTopic(topicId, initialPeers);
  await setTimelineUpdateMode('realtime');
  await openTopic(topicId);
};

const writeDiagnostics = (
  payload: Record<string, unknown>,
  fileName = 'direct-peer-regression.json',
) => {
  const diagnosticsPath = resolve(process.cwd(), '..', 'test-results', 'multi-peer-e2e', fileName);
  mkdirSync(dirname(diagnosticsPath), { recursive: true });
  writeFileSync(diagnosticsPath, JSON.stringify(payload, null, 2), 'utf-8');
};

describe('Direct peer realtime propagation', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('updates timeline in realtime and resolves profile label without reload', async function () {
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
    const connectMode = resolveConnectMode();
    const useDirectJoinHints = shouldUseDirectJoinHints(connectMode);
    const callDirectConnect = shouldCallDirectConnect(connectMode);
    const expectStaleRender = parseOptionalBool(process.env.E2E_DIRECT_PEER_EXPECT_STALE) ?? false;
    const expectProfileUnresolved =
      parseOptionalBool(process.env.E2E_DIRECT_PEER_EXPECT_PROFILE_UNRESOLVED) ?? false;
    const enableSinglePostCase =
      parseOptionalBool(process.env.E2E_DIRECT_PEER_SINGLE_POST_CASE) ?? true;
    const singlePostTimeoutMs = parseTimeoutMs(
      process.env.E2E_DIRECT_PEER_SINGLE_POST_TIMEOUT_MS,
      DEFAULT_DIRECT_PEER_TIMEOUT_MS,
    );
    const expectedSinglePostRendered =
      parseOptionalBool(process.env.E2E_DIRECT_PEER_SINGLE_POST_EXPECT_RENDERED) ?? true;
    const requireStrictSinglePublish =
      parseOptionalBool(process.env.E2E_DIRECT_PEER_SINGLE_POST_REQUIRE_STRICT) ?? false;
    const expectedProfileName = resolveEnvString(
      process.env.E2E_MULTI_PEER_EXPECTED_PROFILE_NAME,
      DEFAULT_EXPECTED_PROFILE_NAME,
    );
    const bootstrapPeers = parseBootstrapPeers();
    const requireZeroPeersBeforeConnect =
      parseOptionalBool(process.env.E2E_DIRECT_PEER_REQUIRE_ZERO_BEFORE_CONNECT) ?? false;
    const expectedPeersBeforeConnect = parseOptionalInt(
      process.env.E2E_DIRECT_PEER_EXPECT_PEERS_BEFORE_CONNECT,
    );
    let bootstrapSnapshot = await getBootstrapSnapshot();
    if (bootstrapPeers.length > 0) {
      await browser.waitUntil(
        async () => {
          bootstrapSnapshot = await getBootstrapSnapshot();
          if (bootstrapSnapshot.effectiveNodes.length === 0) {
            return false;
          }
          return hasExpectedBootstrapNode(bootstrapSnapshot.effectiveNodes, bootstrapPeers);
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

    const peerAddress = await waitForPeerAddress();
    const targetPeerNodeId = parsePeerAddressNodeId(peerAddress);
    const joinInitialPeers = useDirectJoinHints ? [peerAddress] : [];

    await setupTopicPageForDirectPeer(topicId, joinInitialPeers);
    const baselineTimelineSnapshot = await getTimelineThreadSnapshot(publishPrefix);
    const baselinePostStoreSnapshot = await getPostStoreSnapshot(topicId);
    const baselineP2PSnapshot = await getP2PMessageSnapshot(topicId);

    const beforeConnectStatus = await getP2PStatus();
    const beforeConnectPeerCount = getTopicPeerCount(beforeConnectStatus, topicId);
    if (expectedPeersBeforeConnect !== null) {
      expect(beforeConnectPeerCount).toBeGreaterThanOrEqual(expectedPeersBeforeConnect);
    }
    if (requireZeroPeersBeforeConnect) {
      expect(beforeConnectPeerCount).toBe(0);
    }

    if (callDirectConnect) {
      await connectToP2PPeer(peerAddress);
    }

    await browser.waitUntil(
      async () => isTargetPeerConnected(await getP2PStatus(), topicId, targetPeerNodeId),
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: `Peer connection did not become active for ${peerAddress} (mode=${connectMode})`,
      },
    );

    let postStoreAfterSingleArrival: PostStoreSnapshot | null = null;
    let p2pAfterSingleArrival: P2PMessageSnapshot | null = null;
    let timelineSnapshotAfterSingleArrival: TimelineThreadSnapshot | null = null;
    let renderedOnSingleArrivalWithoutLocalAction: boolean | null = null;
    let bodyContainsPrefixWithoutLocalAction: boolean | null = null;
    let singleArrivalContentMarker: string | null = null;
    let strictPublishedCount: number | null = null;
    if (enableSinglePostCase) {
      p2pAfterSingleArrival = await waitForP2PMessageAdvance(
        topicId,
        publishPrefix,
        baselineP2PSnapshot,
        singlePostTimeoutMs,
      );
      postStoreAfterSingleArrival = await getPostStoreSnapshot(topicId);
      singleArrivalContentMarker = pickNewP2PContentMarker(
        baselineP2PSnapshot,
        p2pAfterSingleArrival,
        publishPrefix,
      );
      if (!singleArrivalContentMarker) {
        singleArrivalContentMarker = publishPrefix;
      }
      timelineSnapshotAfterSingleArrival = await getTimelineThreadSnapshot(
        singleArrivalContentMarker,
      );
      renderedOnSingleArrivalWithoutLocalAction = await waitForBodyText(
        singleArrivalContentMarker,
        2000,
      );
      bodyContainsPrefixWithoutLocalAction = await waitForBodyText(publishPrefix, 2000);

      if (requireStrictSinglePublish) {
        const peerSummary = await waitForPeerSummary();
        strictPublishedCount = peerSummary.stats?.published_count ?? 0;
        expect(strictPublishedCount).toBe(1);
      }
      expect(renderedOnSingleArrivalWithoutLocalAction).toBe(expectedSinglePostRendered);
    }

    const p2pSnapshotAfterPropagation = enableSinglePostCase
      ? p2pAfterSingleArrival
      : await waitForP2PMessageAdvance(topicId, publishPrefix, baselineP2PSnapshot, 180000);
    if (!p2pSnapshotAfterPropagation) {
      throw new Error('P2P propagation snapshot is not available');
    }
    const propagationContentMarker =
      singleArrivalContentMarker ??
      pickNewP2PContentMarker(baselineP2PSnapshot, p2pSnapshotAfterPropagation, publishPrefix);
    const resolvedPropagationContentMarker = propagationContentMarker ?? publishPrefix;
    const postStoreSnapshotAfterPropagation = await getPostStoreSnapshot(topicId);

    const withoutReloadTimelineResult = await waitForTimelineSnapshotChange(
      baselineTimelineSnapshot,
      resolvedPropagationContentMarker,
      15000,
    );
    const renderedWithoutReload = await waitForBodyText(resolvedPropagationContentMarker, 15000);
    const bodyContainsPrefixWithoutReload = await waitForBodyText(publishPrefix, 15000);
    if (expectStaleRender) {
      expect(renderedWithoutReload).toBe(false);
    } else {
      expect(renderedWithoutReload).toBe(true);
    }

    await browser.refresh();
    await waitForAppReady();
    await openTopic(topicId);

    const afterReloadTimelineResult = await waitForTimelineSnapshotChange(
      baselineTimelineSnapshot,
      resolvedPropagationContentMarker,
      60000,
    );
    const renderedAfterReload = await waitForBodyText(resolvedPropagationContentMarker, 60000);
    expect(renderedAfterReload).toBe(true);
    const bodyContainsPrefixAfterReload = await waitForBodyText(publishPrefix, 60000);

    const bodyText = await $('body').getText();
    const profileResolved = bodyText.includes(expectedProfileName);
    if (expectProfileUnresolved) {
      expect(profileResolved).toBe(false);
    } else {
      expect(profileResolved).toBe(true);
    }

    const screenshotPath = resolve(
      process.cwd(),
      '..',
      'test-results',
      'multi-peer-e2e',
      'direct-peer-regression.png',
    );
    mkdirSync(dirname(screenshotPath), { recursive: true });
    await browser.saveScreenshot(screenshotPath);

    const afterConnectStatus = await getP2PStatus();
    const afterConnectPeerCount =
      afterConnectStatus.active_topics.find((topic) => topic.topic_id === topicId)?.peer_count ?? 0;
    writeDiagnostics({
      scenario: process.env.SCENARIO ?? null,
      topicId,
      peerAddress,
      targetPeerNodeId,
      connectMode,
      useDirectJoinHints,
      callDirectConnect,
      joinInitialPeers,
      bootstrapSnapshot,
      publishPrefix,
      expectedProfileName,
      expectStaleRender,
      expectProfileUnresolved,
      enableSinglePostCase,
      singleArrivalContentMarker,
      propagationContentMarker: resolvedPropagationContentMarker,
      beforeConnectPeerCount,
      expectedPeersBeforeConnect,
      afterConnectPeerCount,
      singlePostTimeoutMs,
      expectedSinglePostRendered,
      requireStrictSinglePublish,
      strictPublishedCount,
      baselinePostStoreSnapshot,
      postStoreAfterSingleArrival,
      baselineP2PSnapshot,
      p2pSnapshotAfterPropagation,
      postStoreSnapshotAfterPropagation,
      baselineTimelineSnapshot,
      timelineSnapshotAfterSingleArrival,
      timelineSnapshotWithoutReload: withoutReloadTimelineResult.snapshot,
      timelineSnapshotAfterReload: afterReloadTimelineResult.snapshot,
      renderedOnSingleArrivalWithoutLocalAction,
      renderedWithoutReload,
      renderedAfterReload,
      bodyContainsPrefixWithoutLocalAction,
      bodyContainsPrefixWithoutReload,
      bodyContainsPrefixAfterReload,
      profileResolved,
    });
  });
});
