import { $, browser, expect } from '@wdio/globals';
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  applyCliBootstrap,
  communityNodeListBootstrapNodes,
  communityNodeListBootstrapServices,
  getP2PStatus,
  getP2PMessageSnapshot,
  getBootstrapSnapshot,
  getTopicSnapshot,
  joinP2PTopic,
  resetAppState,
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
const REAL_BOOTSTRAP_PEER =
  process.env.E2E_REAL_BOOTSTRAP_PEER ??
  '03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8@127.0.0.1:11233';
const REAL_BOOTSTRAP_NODE_ID = REAL_BOOTSTRAP_PEER.split('@')[0];

const profile: ProfileInfo = {
  name: 'E2E CN Relay',
  displayName: 'community-node-cn-cli-propagation',
  about: 'Community node bootstrap/relay + cn-cli publish propagation',
};

type SeedPostSummary = {
  topic_id?: string;
  content?: string;
};

type SeedSummary = {
  post?: SeedPostSummary;
};

const parseSeedSummary = (): SeedSummary | null => {
  const raw = process.env.E2E_COMMUNITY_NODE_SEED_JSON;
  if (!raw) {
    return null;
  }
  try {
    return JSON.parse(raw) as SeedSummary;
  } catch {
    return null;
  }
};

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

const findEventByKind = (items: unknown[], kind: number): Record<string, unknown> | null => {
  for (const item of items) {
    if (item && typeof item === 'object' && (item as { kind?: number }).kind === kind) {
      return item as Record<string, unknown>;
    }
  }
  return null;
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

const runCnCliPublish = (content: string, topicId: string, peer: string) => {
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
    content,
    '--repeat',
    '6',
    '--interval-ms',
    '400',
    '--wait-for-peer-secs',
    '8',
    '--peers',
    peer,
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
      `cn-cli publish failed (status=${result.status}): ${result.stderr || result.stdout}`,
    );
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
    const seed = parseSeedSummary();
    const bootstrapServiceTopicId = seed?.post?.topic_id ?? 'kukuri:e2e-alpha';

    if (!baseUrl || scenario !== 'community-node-e2e') {
      this.skip();
      return;
    }

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await runCommunityNodeAuthFlow(baseUrl);
    const topicSnapshot = await getTopicSnapshot();
    const propagationTopicId = topicSnapshot.currentTopicId?.trim() || DEFAULT_PUBLIC_TOPIC_ID;

    let nodesPayload: Record<string, unknown> = {};
    await browser.waitUntil(
      async () => {
        nodesPayload = await communityNodeListBootstrapNodes();
        return extractItems(nodesPayload).length > 0;
      },
      {
        timeout: 40000,
        interval: 1000,
        timeoutMsg: 'Community node bootstrap nodes did not return any items',
      },
    );
    const nodeItems = extractItems(nodesPayload);
    const descriptorEvent =
      findEventByKind(nodeItems, 39000) ?? (nodeItems[0] as Record<string, unknown>);
    const descriptorContent = parseEventContent(descriptorEvent);
    expect(descriptorContent?.schema).toBe('kukuri-node-desc-v1');
    const roles = Array.isArray(descriptorContent?.roles) ? descriptorContent?.roles : [];
    expect(roles).toContain('bootstrap');
    expect(roles).toContain('relay');

    let servicesPayload: Record<string, unknown> = {};
    await browser.waitUntil(
      async () => {
        servicesPayload = await communityNodeListBootstrapServices(bootstrapServiceTopicId);
        return extractItems(servicesPayload).length > 0;
      },
      {
        timeout: 40000,
        interval: 1000,
        timeoutMsg: 'Community node bootstrap services did not return any items',
      },
    );
    const serviceItems = extractItems(servicesPayload);
    const serviceEvent =
      findEventByKind(serviceItems, 39001) ?? (serviceItems[0] as Record<string, unknown>);
    const serviceContent = parseEventContent(serviceEvent);
    expect(serviceContent?.schema).toBe('kukuri-topic-service-v1');
    expect(['relay', 'bootstrap']).toContain(serviceContent?.role);

    const timelineScreenshotPath =
      process.env.E2E_TIMELINE_SCREENSHOT_PATH ??
      resolve(
        process.cwd(),
        '..',
        'test-results',
        'community-node-e2e',
        'cn-cli-propagation-timeline.png',
      );

    const postsList = await $('[data-testid="posts-list"]');
    await postsList.waitForDisplayed({ timeout: 30000 });

    const seedPostContent = seed?.post?.content?.trim();
    if (seedPostContent) {
      try {
        await browser.waitUntil(
          async () => (await postsList.getText()).includes(seedPostContent),
          {
            timeout: 60000,
            interval: 1000,
            timeoutMsg: 'Seed event did not appear on timeline',
          },
        );
      } catch {
        console.info('[community-node.cn-cli-propagation] seed event was not visible before capture');
      }
    }
    mkdirSync(dirname(timelineScreenshotPath), { recursive: true });
    await browser.saveScreenshot(timelineScreenshotPath);
    console.info(
      `[community-node.cn-cli-propagation] timeline screenshot (seed): ${timelineScreenshotPath}`,
    );

    writeCliBootstrapFixture([REAL_BOOTSTRAP_PEER]);
    await applyCliBootstrap();

    await browser.waitUntil(
      async () => {
        const snapshot = await getBootstrapSnapshot();
        return (
          snapshot.source === 'user' &&
          snapshot.effectiveNodes.length > 0 &&
          snapshot.effectiveNodes.includes(REAL_BOOTSTRAP_PEER)
        );
      },
      {
        timeout: 40000,
        interval: 1000,
        timeoutMsg: 'Effective bootstrap nodes were not updated for propagation test',
      },
    );

    const ensureBootstrapConnected = async (timeoutMs: number) => {
      await browser.waitUntil(
        async () => {
          const status = await getP2PStatus();
          const topicStatus = status.active_topics.find((topic) => topic.topic_id === propagationTopicId);
          const isConnectedToBootstrap = status.peers.some(
            (peer) => peer.node_id === REAL_BOOTSTRAP_NODE_ID,
          );
          const connected =
            Boolean(topicStatus) &&
            (topicStatus?.peer_count ?? 0) > 0 &&
            status.peer_count > 0 &&
            isConnectedToBootstrap;
          if (!connected) {
            await joinP2PTopic(propagationTopicId, [REAL_BOOTSTRAP_PEER]);
            return false;
          }
          return true;
        },
        {
          timeout: timeoutMs,
          interval: 1000,
          timeoutMsg: 'P2P topic did not join bootstrap/relay peer for propagation test',
        },
      );
    };

    await joinP2PTopic(propagationTopicId, [REAL_BOOTSTRAP_PEER]);
    await ensureBootstrapConnected(40000);

    const statusBeforePublish = await getP2PStatus();
    const baselineMessagesReceived = statusBeforePublish.metrics_summary.messages_received;

    let contentPrefix = '';
    let received = false;
    for (let attempt = 1; attempt <= 3; attempt += 1) {
      contentPrefix = `E2E cn-cli -> tauri propagation ${Date.now()} [attempt:${attempt}]`;
      runCnCliPublish(contentPrefix, propagationTopicId, REAL_BOOTSTRAP_PEER);
      try {
        await browser.waitUntil(
          async () => {
            const status = await getP2PStatus();
            return status.metrics_summary.messages_received > baselineMessagesReceived;
          },
          {
            timeout: 45000,
            interval: 1000,
            timeoutMsg: 'Tauri did not receive cn-cli published gossip message',
          },
        );
        received = true;
        break;
      } catch {
        await ensureBootstrapConnected(30000);
      }
    }
    expect(received).toBe(true);

    let p2pSnapshot = await getP2PMessageSnapshot(propagationTopicId);
    await browser.waitUntil(
      async () => {
        p2pSnapshot = await getP2PMessageSnapshot(propagationTopicId);
        return p2pSnapshot.count > 0;
      },
      {
        timeout: 20000,
        interval: 1000,
        timeoutMsg: `P2P bridge did not receive topic message: ${JSON.stringify(p2pSnapshot)}`,
      },
    );

    await browser.waitUntil(
      async () => (await postsList.getText()).includes(contentPrefix),
      {
        timeout: 120000,
        interval: 1000,
        timeoutMsg: 'cn-cli published content did not appear on Tauri timeline',
      },
    );

    await browser.saveScreenshot(timelineScreenshotPath);
    console.info(`[community-node.cn-cli-propagation] timeline screenshot: ${timelineScreenshotPath}`);
  });
});
