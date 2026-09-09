import { expect, type Locator, type Page } from '@playwright/test';

import { columnIdentityId } from '../../src/shell/slices/workspace';
import { WORKSPACE_LAYOUT_STORAGE_KEY } from '../../src/shell/workspacePersistence';

export const LONG_NODE_URLS = [
  `https://${'community-node-'.repeat(3)}first.discovery.service.example.test`,
  `https://${'community-node-'.repeat(3)}second.discovery.service.example.test`,
];

export async function seedIndexLayout(
  page: Page,
  { locale = 'ja', theme = 'dark', pendingNodes = false, threeColumns = true, holdQueries = false } = {}
) {
  const scope = { topicId: 'kukuri:topic:general', channelId: null };
  const columns = (['timeline', 'notifications', 'explore'] as const).map((kind) => ({
    id: columnIdentityId(kind, scope), kind, scope, pinned: true, preferredDesktopSpan: 1,
  }));
  await page.addInitScript(({ locale, theme, pendingNodes, threeColumns, holdQueries, urls, key, columns }) => {
    localStorage.setItem('kukuri.desktop.locale', locale);
    localStorage.setItem('kukuri.desktop.theme', theme);
    if (threeColumns) {
      localStorage.setItem(key, JSON.stringify({ version: 1, activeColumnId: columns[2].id, columns }));
    }
    const calls: { method: string; baseUrl: string }[] = [];
    Object.defineProperty(window, '__indexLayoutCalls', { value: calls });
    let desktopApi = window.__KUKURI_DESKTOP__;
    Object.defineProperty(window, '__KUKURI_DESKTOP__', {
      configurable: true,
      get: () => desktopApi,
      set: (api: typeof desktopApi) => {
        desktopApi = api;
        if (!api) return;
        if (pendingNodes) {
          void api.setCommunityNodeConfig([
            { base_url: 'https://api.kukuri.app' }, ...urls.map((base_url) => ({ base_url })),
          ]);
        }
        for (const method of ['searchCommunityNodeIndex', 'discoverCommunityNodeIndex', 'recommendCommunityNodeIndex'] as const) {
          const original = api[method].bind(api);
          api[method] = async (request) => {
            calls.push({ method, baseUrl: request.base_url });
            if (holdQueries) {
              await new Promise<void>((resolve) => {
                Object.defineProperty(window, '__releaseIndexLayoutQuery', { configurable: true, value: resolve });
              });
            }
            return original(request);
          };
        }
        const policies = api.fetchCommunityNodePolicies.bind(api);
        api.fetchCommunityNodePolicies = async (baseUrl, language) => {
          calls.push({ method: 'policies', baseUrl });
          return policies(baseUrl, language);
        };
        const accept = api.acceptCommunityNodeConsents.bind(api);
        api.acceptCommunityNodeConsents = async (...args) => {
          calls.push({ method: 'accept', baseUrl: args[0] });
          return accept(...args);
        };
      },
    });
  }, { locale, theme, pendingNodes, threeColumns, holdQueries, urls: LONG_NODE_URLS, key: WORKSPACE_LAYOUT_STORAGE_KEY, columns });
}

export async function indexLayoutCalls(page: Page) {
  return page.evaluate(() => (window as unknown as {
    __indexLayoutCalls: { method: string; baseUrl: string }[];
  }).__indexLayoutCalls);
}

// Document overflow / isVisible alone do not detect text clipped inside a Column.
export async function expectIndexContentContained(workspace: Locator) {
  const issues = await workspace.evaluate((root) => {
    const bounds = root.getBoundingClientRect();
    const problems: string[] = [];
    if (root.scrollWidth > root.clientWidth + 1) problems.push('workspace scrolls horizontally');
    for (const control of root.querySelectorAll('button, input')) {
      const rect = control.getBoundingClientRect();
      if (rect.width && (rect.left < bounds.left - 1 || rect.right > bounds.right + 1)) {
        problems.push(`control outside workspace: ${control.textContent?.trim()}`);
      }
    }
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let node = walker.nextNode();
    while (node) {
      if (node.textContent?.trim()) {
        const range = document.createRange();
        range.selectNodeContents(node);
        const control = node.parentElement?.closest('button');
        const controlBounds = control?.getBoundingClientRect();
        for (const rect of range.getClientRects()) {
          if (!rect.width || !rect.height) continue;
          if (rect.left < bounds.left - 1 || rect.right > bounds.right + 1) {
            problems.push(`text outside workspace: ${node.textContent.slice(0, 90)}`);
          }
          if (controlBounds && (rect.left < controlBounds.left - 1 || rect.right > controlBounds.right + 1 || rect.top < controlBounds.top - 1 || rect.bottom > controlBounds.bottom + 1)) {
            problems.push(`text outside button: ${node.textContent.slice(0, 90)}`);
          }
        }
      }
      node = walker.nextNode();
    }
    return [...new Set(problems)];
  });
  expect(issues).toEqual([]);
}
