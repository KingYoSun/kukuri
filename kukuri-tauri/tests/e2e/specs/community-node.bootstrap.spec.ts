import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady';
import {
  resetAppState,
  communityNodeListBootstrapNodes,
  communityNodeListBootstrapServices,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { runCommunityNodeAuthFlow } from '../helpers/communityNodeAuth';

type SeedPostSummary = {
  topic_id?: string;
};

type SeedSummary = {
  post?: SeedPostSummary;
};

const profile: ProfileInfo = {
  name: 'E2E Community',
  displayName: 'community-node-bootstrap',
  about: 'Community Node bootstrap/relay endpoint flow',
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

describe('Community Node bootstrap/relay endpoints', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('loads bootstrap nodes/services from the real community node', async function () {
    this.timeout(240000);

    const baseUrl = process.env.E2E_COMMUNITY_NODE_URL;
    const scenario = process.env.SCENARIO;
    const seed = parseSeedSummary();
    const topicId = seed?.post?.topic_id ?? 'kukuri:e2e-alpha';

    if (!baseUrl || scenario !== 'community-node-e2e' || !topicId) {
      this.skip();
      return;
    }

    await waitForWelcome();
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    await runCommunityNodeAuthFlow(baseUrl);

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
    expect(nodeItems.length).toBeGreaterThan(0);
    const descriptorEvent =
      findEventByKind(nodeItems, 39000) ?? (nodeItems[0] as Record<string, unknown>);
    const descriptorContent = parseEventContent(descriptorEvent);
    expect(descriptorContent?.schema).toBe('kukuri-node-desc-v1');
    const roles = Array.isArray(descriptorContent?.roles) ? descriptorContent?.roles : [];
    expect(roles).toContain('bootstrap');
    expect(roles).toContain('relay');
    const endpoints = descriptorContent?.endpoints as Record<string, unknown> | undefined;
    expect(Boolean(endpoints?.http || endpoints?.ws)).toBe(true);

    let servicesPayload: Record<string, unknown> = {};
    await browser.waitUntil(
      async () => {
        servicesPayload = await communityNodeListBootstrapServices(topicId);
        return extractItems(servicesPayload).length > 0;
      },
      {
        timeout: 40000,
        interval: 1000,
        timeoutMsg: 'Community node bootstrap services did not return any items',
      },
    );

    const serviceItems = extractItems(servicesPayload);
    expect(serviceItems.length).toBeGreaterThan(0);
    const serviceEvent =
      findEventByKind(serviceItems, 39001) ?? (serviceItems[0] as Record<string, unknown>);
    const serviceContent = parseEventContent(serviceEvent);
    expect(serviceContent?.schema).toBe('kukuri-topic-service-v1');
    expect(serviceContent?.topic).toBe(topicId);
    expect(['relay', 'bootstrap']).toContain(serviceContent?.role);
    expect(['public', 'friend_plus', 'friend', 'invite']).toContain(serviceContent?.scope);
  });
});
