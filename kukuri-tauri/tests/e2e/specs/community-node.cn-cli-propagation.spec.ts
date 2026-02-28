import { $, $$, browser, expect } from '@wdio/globals';
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  applyCliBootstrap,
  communityNodeListBootstrapNodes,
  ensureTestTopic,
  getBootstrapSnapshot,
  getP2PMessageSnapshot,
  getP2PStatus,
  getPostStoreSnapshot,
  getTimelineUpdateMode,
  joinP2PTopic,
  resetAppState,
  seedCommunityNodePost,
  setTimelineUpdateMode,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

const DEFAULT_PUBLIC_TOPIC_ID =
  'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0';
const DEFAULT_BOOTSTRAP_PEER =
  '03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233';
const OVERRIDE_BOOTSTRAP_PEERS = (process.env.E2E_REAL_BOOTSTRAP_PEER ?? '')
  .split(',')
  .map((value) => value.trim())
  .filter((value) => value.length > 0);

const profile: ProfileInfo = {
  name: 'E2E CN Relay',
  displayName: 'community-node-cn-cli-propagation',
  about: 'Community node bootstrap/relay + cn-cli publish propagation',
};

const writeCliBootstrapFixture = (peers: string[]) => {
  const bootstrapPath = process.env.KUKURI_CLI_BOOTSTRAP_PATH ?? process.env.KUKURI_P2P_BOOTSTRAP_PATH;
  if (!bootstrapPath) {
    throw new Error('KUKURI_CLI_BOOTSTRAP_PATH/KUKURI_P2P_BOOTSTRAP_PATH is not set');
  }
  mkdirSync(dirname(bootstrapPath), { recursive: true });
  const payload = {
    nodes: peers,
    updated_at_ms: Date.now(),
  };
  writeFileSync(bootstrapPath, JSON.stringify(payload, null, 2), 'utf-8');
};

type PublishSummary = {
  topic_id?: string;
  published_count?: number;
  event_ids?: string[];
};

const MAX_PROPAGATION_ATTEMPTS = 2;
const PROPAGATION_WAIT_TIMEOUT_MS = 30000;
const TIMELINE_RENDER_TIMEOUT_MS = 30000;
const TOPIC_PAGE_RENDER_FALLBACK_TIMEOUT_MS = 20000;

const extractItems = (payload: Record<string, unknown>): unknown[] => {
  const items = payload?.items;
  return Array.isArray(items) ? items : [];
};

const parseEventContent = (event: unknown): Record<string, unknown> | null => {
  if (!event || typeof event !== 'object') {
    return null;
  }
  const content = (event as { content?: unknown }).content;
  if (typeof content !== 'string' || content.trim().length === 0) {
    return null;
  }
  try {
    return JSON.parse(content) as Record<string, unknown>;
  } catch {
    return null;
  }
};

const normalizeBootstrapNodeList = (value: unknown): string[] =>
  Array.isArray(value)
    ? value
        .filter((item): item is string => typeof item === 'string')
        .map((item) => item.trim())
        .filter((item) => item.length > 0 && item.includes('@'))
    : [];

const extractBootstrapNodes = (content: Record<string, unknown>): string[] => {
  const directNodes = normalizeBootstrapNodeList(content.bootstrap_nodes);
  const endpoints = content.endpoints;
  if (!endpoints || typeof endpoints !== 'object') {
    return directNodes;
  }
  const p2p = (endpoints as Record<string, unknown>).p2p;
  if (!p2p || typeof p2p !== 'object') {
    return directNodes;
  }
  const endpointNodes = normalizeBootstrapNodeList((p2p as Record<string, unknown>).bootstrap_nodes);
  return [...directNodes, ...endpointNodes];
};

const dedupeBootstrapNodes = (nodes: string[]): string[] => {
  const seen = new Set<string>();
  const deduped: string[] = [];
  for (const node of nodes) {
    if (seen.has(node)) {
      continue;
    }
    seen.add(node);
    deduped.push(node);
  }
  return deduped;
};

const extractNodeId = (node: string): string | null => {
  const normalized = node.trim();
  if (normalized.length === 0) {
    return null;
  }
  const separatorIndex = normalized.indexOf('@');
  if (separatorIndex <= 0) {
    return null;
  }
  return normalized.slice(0, separatorIndex);
};

const resolveBootstrapPeers = async (): Promise<string[]> => {
  try {
    const payload = await communityNodeListBootstrapNodes();
    const items = extractItems(payload);
    const resolved: string[] = [];
    resolved.push(...normalizeBootstrapNodeList(payload.bootstrap_nodes));
    for (const item of items) {
      const content = parseEventContent(item);
      if (!content) {
        continue;
      }
      resolved.push(...extractBootstrapNodes(content));
    }
    const deduped = dedupeBootstrapNodes(resolved);
    if (deduped.length > 0) {
      return deduped;
    }
  } catch (error) {
    console.info(
      '[community-node.cn-cli-propagation] failed to resolve runtime bootstrap peers',
      error instanceof Error ? error.message : String(error),
    );
  }
  if (OVERRIDE_BOOTSTRAP_PEERS.length > 0) {
    return OVERRIDE_BOOTSTRAP_PEERS;
  }
  return [DEFAULT_BOOTSTRAP_PEER];
};

const resolveCanonicalTopicId = (topicId: string): string => {
  const workspace = resolve(process.cwd(), '..', 'kukuri-community-node');
  const args = [
    'run',
    '--locked',
    '-p',
    'cn-cli',
    '--',
    'p2p',
    'publish',
    '--bind',
    '127.0.0.1:0',
    '--log-level',
    'error',
    '--topic',
    topicId,
    '--content',
    `E2E canonical topic probe ${Date.now()}`,
    '--repeat',
    '1',
    '--interval-ms',
    '0',
    '--wait-for-peer-secs',
    '0',
    '--no-dht',
  ];
  const result = spawnSync('cargo', args, {
    cwd: workspace,
    encoding: 'utf-8',
    env: {
      ...process.env,
      RUST_LOG: 'error',
    },
  });
  if (result.status !== 0) {
    throw new Error(
      `cn-cli topic canonicalization failed (status=${result.status}): ${result.stderr || result.stdout}`,
    );
  }
  const stdout = result.stdout?.trim();
  if (!stdout) {
    throw new Error('cn-cli topic canonicalization returned empty stdout');
  }
  const lastLine = stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .slice(-1)[0];
  if (!lastLine) {
    throw new Error('cn-cli topic canonicalization summary line was missing');
  }
  const summary = JSON.parse(lastLine) as PublishSummary;
  const canonicalTopicId = summary.topic_id?.trim();
  if (!canonicalTopicId) {
    throw new Error(`cn-cli topic canonicalization did not return topic_id: ${lastLine}`);
  }
  return canonicalTopicId;
};

const runCnCliPublish = (content: string, topicId: string, peers: string[]): PublishSummary | null => {
  const workspace = resolve(process.cwd(), '..', 'kukuri-community-node');
  const args: string[] = [
    'run',
    '--locked',
    '-p',
    'cn-cli',
    '--',
    'p2p',
    'publish',
    '--bind',
    '0.0.0.0:0',
    '--log-level',
    'error',
    '--topic',
    topicId,
    '--content',
    content,
    '--repeat',
    '4',
    '--interval-ms',
    '250',
    '--wait-for-peer-secs',
    '5',
  ];
  for (const peer of peers) {
    args.push('--peers', peer);
  }
  const result = spawnSync('cargo', args, {
    cwd: workspace,
    encoding: 'utf-8',
    env: {
      ...process.env,
      RUST_LOG: 'error',
    },
  });
  if (result.status !== 0) {
    throw new Error(
      `cn-cli publish failed (status=${result.status}): ${result.stderr || result.stdout}`,
    );
  }

  const stdout = result.stdout?.trim();
  if (!stdout) {
    return null;
  }
  const lastLine = stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .slice(-1)[0];
  if (!lastLine) {
    return null;
  }
  try {
    return JSON.parse(lastLine) as PublishSummary;
  } catch {
    return null;
  }
};

describe('Community Node bootstrap/relay + cn-cli propagation', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('receives cn-cli published event on Tauri client and renders it', async function () {
    this.timeout(480000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const scenario = process.env.SCENARIO;

    if (!baseUrl || scenario !== 'community-node-e2e') {
      this.skip();
      return;
    }
    const propagationTopicId = resolveCanonicalTopicId(DEFAULT_PUBLIC_TOPIC_ID);
    console.info(
      `[community-node.cn-cli-propagation] propagationTopicId:${propagationTopicId}`,
    );

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await runCommunityNodeAuthFlow(baseUrl);
    const bootstrapPeers = await resolveBootstrapPeers();
    console.info(
      `[community-node.cn-cli-propagation] bootstrapPeers:${JSON.stringify(bootstrapPeers)}`,
    );
    await ensureTestTopic({
      name: 'community-node-cn-cli-propagation',
      topicId: propagationTopicId,
    });

    const timelineScreenshotPath =
      process.env.E2E_TIMELINE_SCREENSHOT_PATH ??
      resolve(
        process.cwd(),
        '..',
        'test-results',
        'community-node-e2e',
        'cn-cli-propagation-received-timeline.png',
      );

    writeCliBootstrapFixture(bootstrapPeers);
    await applyCliBootstrap();

    await browser.waitUntil(
      async () => {
        const snapshot = await getBootstrapSnapshot();
        return (
          snapshot.source === 'user' &&
          snapshot.effectiveNodes.length > 0 &&
          snapshot.effectiveNodes.some((node) => bootstrapPeers.includes(node))
        );
      },
      {
        timeout: 40000,
        interval: 1000,
        timeoutMsg: 'Effective bootstrap nodes were not updated for propagation test',
      },
    );

    const bootstrapNodeIds = bootstrapPeers
      .map((node) => extractNodeId(node))
      .filter((nodeId): nodeId is string => Boolean(nodeId));
    await joinP2PTopic(propagationTopicId, bootstrapPeers);
    await browser.waitUntil(
      async () => {
        const status = await getP2PStatus();
        const topicStatus = status.active_topics.find((topic) => topic.topic_id === propagationTopicId);
        const connectedBootstrapPeer =
          bootstrapNodeIds.length === 0 ||
          status.peers.some((peer) => bootstrapNodeIds.includes(peer.node_id));
        return Boolean(topicStatus) && status.connection_status === 'connected' && connectedBootstrapPeer;
      },
      {
        timeout: 30000,
        interval: 1000,
        timeoutMsg: 'P2P topic did not become active and connected for propagation test',
      },
    );
    await waitForHome();
    await setTimelineUpdateMode('realtime');

    // Realtime delta を適用する timeline は topic detail で監視されるため、先に遷移しておく。
    const topicSidebarButton = await $(`[data-testid="topic-${propagationTopicId}"]`);
    await topicSidebarButton.waitForDisplayed({ timeout: 20000 });
    await topicSidebarButton.click();
    await browser.waitUntil(
      async () => {
        const currentUrl = decodeURIComponent(await browser.getUrl());
        return currentUrl.includes(`/topics/${propagationTopicId}`);
      },
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: 'Propagation topic detail did not open before publish',
      },
    );
    const publishPeers = bootstrapPeers;
    console.info(
      `[community-node.cn-cli-propagation] publishPeers:${JSON.stringify(publishPeers)}`,
    );

    const baselineSnapshot = await getP2PMessageSnapshot(propagationTopicId);
    const baselinePostSnapshot = await getPostStoreSnapshot(propagationTopicId);
    const baselineStatus = await getP2PStatus();

    let contentPrefix = '';
    let received = false;
    let usedBridgeSeedFallback = false;
    let lastP2PSnapshot = baselineSnapshot;
    let lastPostStoreSnapshot = baselinePostSnapshot;
    let lastStatus = baselineStatus;
    for (let attempt = 1; attempt <= MAX_PROPAGATION_ATTEMPTS; attempt += 1) {
      contentPrefix = `E2E cn-cli -> tauri propagation ${Date.now()} [attempt:${attempt}]`;
      const publishSummary = runCnCliPublish(
        contentPrefix,
        propagationTopicId,
        publishPeers,
      );
      console.info(
        `[community-node.cn-cli-propagation] cn-cli publish attempt:${attempt} summary:${JSON.stringify(
          publishSummary,
        )}`,
      );
      try {
        await browser.waitUntil(
          async () => {
            lastP2PSnapshot = await getP2PMessageSnapshot(propagationTopicId);
            lastPostStoreSnapshot = await getPostStoreSnapshot(propagationTopicId);
            lastStatus = await getP2PStatus();
            const p2pMatched = lastP2PSnapshot.recentContents.some((content) =>
              content.includes(contentPrefix),
            );
            const postStoreMatched = lastPostStoreSnapshot.recentContents.some((content) =>
              content.includes(contentPrefix),
            );
            return p2pMatched || postStoreMatched;
          },
          {
            timeout: PROPAGATION_WAIT_TIMEOUT_MS,
            interval: 1000,
            timeoutMsg: `Tauri did not receive cn-cli payload for attempt:${attempt}`,
          },
        );
        received = true;
        break;
      } catch (error) {
        console.info(
          `[community-node.cn-cli-propagation] receive wait failed attempt:${attempt}:${
            error instanceof Error ? error.message : String(error)
          }`,
        );
        try {
          await joinP2PTopic(propagationTopicId, publishPeers);
        } catch (joinError) {
          console.info(
            `[community-node.cn-cli-propagation] rejoin failed attempt:${attempt}:${
              joinError instanceof Error ? joinError.message : String(joinError)
            }`,
          );
        }
      }
    }
    if (!received) {
      usedBridgeSeedFallback = true;
      contentPrefix = `${contentPrefix} [bridge-fallback]`;
      await seedCommunityNodePost({
        id: `e2e-cn-cli-fallback-${Date.now()}`,
        content: contentPrefix,
        authorPubkey: 'f'.repeat(64),
        topicId: propagationTopicId,
        createdAt: Math.floor(Date.now() / 1000),
      });
      lastPostStoreSnapshot = await getPostStoreSnapshot(propagationTopicId);
      console.info('[community-node.cn-cli-propagation] used bridge fallback seed for timeline check');
    }
    if (usedBridgeSeedFallback) {
      console.info(
        '[community-node.cn-cli-propagation] bridge fallback was used only for debug evidence',
      );
    }
    expect(received).toBe(true);
    console.info(
      `[community-node.cn-cli-propagation] P2P snapshot baseline:${JSON.stringify(
        baselineSnapshot,
      )} last:${JSON.stringify(lastP2PSnapshot)} postStoreBaseline:${JSON.stringify(
        baselinePostSnapshot,
      )} postStoreLast:${JSON.stringify(lastPostStoreSnapshot)} statusBaseline:${JSON.stringify(
        baselineStatus.metrics_summary,
      )} statusLast:${JSON.stringify(lastStatus.metrics_summary)}`,
    );

    const findTimelineElementWithContent = async (needle: string) => {
      const threadCards = await $$('[data-testid^="timeline-thread-card-"]');
      for (const threadCard of threadCards) {
        try {
          if (!(await threadCard.isExisting())) {
            continue;
          }
          if ((await threadCard.getText()).includes(needle)) {
            return threadCard;
          }
        } catch {
          continue;
        }
      }

      const timelineItems = await $$('[data-testid^="post-"]');
      for (const item of timelineItems) {
        try {
          if (!(await item.isExisting())) {
            continue;
          }
          if ((await item.getText()).includes(needle)) {
            return item;
          }
        } catch {
          continue;
        }
      }

      const postsLists = await $$('[data-testid="posts-list"]');
      for (const postsList of postsLists) {
        try {
          if (!(await postsList.isExisting())) {
            continue;
          }
          if ((await postsList.getText()).includes(needle)) {
            return postsList;
          }
        } catch {
          continue;
        }
      }

      return null;
    };

    const isPayloadVisibleOnTopicPage = async (needle: string): Promise<boolean> => {
      return await browser.execute(
        (contentNeedle) => (document.body?.innerText ?? '').includes(contentNeedle),
        needle,
      );
    };

    const collectTopicPageDiagnostics = async (needle: string) => {
      let timelineMode: 'standard' | 'realtime' | 'unknown';
      try {
        const modeSnapshot = await getTimelineUpdateMode();
        timelineMode = modeSnapshot.mode;
      } catch {
        timelineMode = 'unknown';
      }

      const threadCards = await $$('[data-testid^="timeline-thread-card-"]');
      const timelineItems = await $$('[data-testid^="post-"]');
      const postsLists = await $$('[data-testid="posts-list"]');

      return {
        url: decodeURIComponent(await browser.getUrl()),
        timelineMode,
        threadCardCount: threadCards.length,
        timelineItemCount: timelineItems.length,
        postsListCount: postsLists.length,
        bodyContainsPayload: await isPayloadVisibleOnTopicPage(needle),
      };
    };

    if (received) {
      let matchedTimelineElement = null;
      let matchedViaTopicPageFallback = false;
      try {
        await browser.waitUntil(
          async () => {
            matchedTimelineElement = await findTimelineElementWithContent(contentPrefix);
            return Boolean(matchedTimelineElement);
          },
          {
            timeout: TIMELINE_RENDER_TIMEOUT_MS,
            interval: 1000,
            timeoutMsg: 'cn-cli received payload did not render on Tauri timeline',
          },
        );
      } catch (timelineError) {
        const diagnostics = await collectTopicPageDiagnostics(contentPrefix);
        console.info(
          `[community-node.cn-cli-propagation] timeline selector wait failed:${
            timelineError instanceof Error ? timelineError.message : String(timelineError)
          } diagnostics:${JSON.stringify(diagnostics)}`,
        );

        await browser.waitUntil(
          async () => {
            matchedViaTopicPageFallback = await isPayloadVisibleOnTopicPage(contentPrefix);
            return matchedViaTopicPageFallback;
          },
          {
            timeout: TOPIC_PAGE_RENDER_FALLBACK_TIMEOUT_MS,
            interval: 1000,
            timeoutMsg: 'cn-cli received payload did not render on Tauri topic page',
          },
        );
      }

      expect(Boolean(matchedTimelineElement) || matchedViaTopicPageFallback).toBe(true);

      if (matchedTimelineElement) {
        await matchedTimelineElement.scrollIntoView({ block: 'center', inline: 'center' });
      }
      if (matchedViaTopicPageFallback && !matchedTimelineElement) {
        console.info(
          '[community-node.cn-cli-propagation] payload detected via topic page text fallback (timeline selectors were empty)',
        );
      }
      console.info(
        `[community-node.cn-cli-propagation] topic page diagnostics:${JSON.stringify(
          await collectTopicPageDiagnostics(contentPrefix),
        )}`,
      );
    } else {
      console.info(
        '[community-node.cn-cli-propagation] skipped timeline render assertion because bridge fallback path was used',
      );
    }

    mkdirSync(dirname(timelineScreenshotPath), { recursive: true });
    await browser.saveScreenshot(timelineScreenshotPath);
    console.info(
      `[community-node.cn-cli-propagation] timeline screenshot (cn-cli received/fallback): ${timelineScreenshotPath}`,
    );
  });
});
