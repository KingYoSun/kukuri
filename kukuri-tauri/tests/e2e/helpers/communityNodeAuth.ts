import { $, browser } from '@wdio/globals';

import { openSettings } from './appActions';

type CommunityNodeSnapshot = {
  location: string;
  auth: string | null;
  lastLog: string | null;
  pageError: string | null;
  bodyText: string;
  nodes: Array<{
    index: number;
    text: string;
    hasToken: string | null;
    pubkey: string | null;
  }>;
};

const normalizeUrl = (value: string): string => value.trim().replace(/\/+$/, '');

const captureCommunityNodeSnapshot = async (): Promise<CommunityNodeSnapshot> => {
  return await browser.execute(() => {
    const doc = document.documentElement;
    const nodes = Array.from(document.querySelectorAll('[data-testid^="community-node-node-"]'))
      .map((node, index) => {
        const tokenStatus = node.querySelector('[data-testid^="community-node-token-status-"]');
        return {
          index,
          text: node.textContent ?? '',
          hasToken: tokenStatus?.getAttribute('data-has-token') ?? null,
          pubkey: tokenStatus?.getAttribute('data-pubkey') ?? null,
        };
      });

    return {
      location: window.location.pathname,
      auth: doc?.getAttribute('data-e2e-auth') ?? null,
      lastLog: doc?.getAttribute('data-e2e-last-log') ?? null,
      pageError: doc?.getAttribute('data-kukuri-e2e-error') ?? null,
      bodyText: document.body?.innerText?.slice(0, 2000) ?? '',
      nodes,
    };
  });
};

const formatSnapshot = (snapshot: CommunityNodeSnapshot): string =>
  JSON.stringify(
    {
      location: snapshot.location,
      auth: snapshot.auth,
      lastLog: snapshot.lastLog,
      pageError: snapshot.pageError,
      nodes: snapshot.nodes,
      bodyText: snapshot.bodyText,
    },
    null,
    2,
  );

const findNodeIndex = (
  snapshot: CommunityNodeSnapshot,
  normalizedBaseUrl: string,
): number | undefined => {
  return snapshot.nodes.find((node) => node.text.includes(normalizedBaseUrl))?.index;
};

const ensureSettingsOpen = async (): Promise<void> => {
  const settingsPage = await $('[data-testid="settings-page"]');
  const isVisible =
    (await settingsPage.isExisting()) &&
    (await settingsPage.isDisplayed().catch(() => false));
  if (!isVisible) {
    await openSettings();
  }
};

const waitForNodeIndex = async (
  normalizedBaseUrl: string,
  timeoutMsg: string,
): Promise<number> => {
  let matchedIndex: number | undefined;
  await browser.waitUntil(
    async () => {
      const snapshot = await captureCommunityNodeSnapshot();
      matchedIndex = findNodeIndex(snapshot, normalizedBaseUrl);
      return matchedIndex !== undefined;
    },
    {
      timeout: 30000,
      interval: 300,
      timeoutMsg,
    },
  );

  if (matchedIndex === undefined) {
    throw new Error(timeoutMsg);
  }

  return matchedIndex;
};

export async function runCommunityNodeAuthFlow(baseUrl: string): Promise<void> {
  const normalizedBaseUrl = normalizeUrl(baseUrl);
  if (!normalizedBaseUrl) {
    throw new Error('Community node base URL is required');
  }

  await ensureSettingsOpen();

  let snapshot = await captureCommunityNodeSnapshot();
  let nodeIndex = findNodeIndex(snapshot, normalizedBaseUrl);

  if (nodeIndex === undefined) {
    const baseInput = await $('[data-testid="community-node-base-url"]');
    await baseInput.waitForDisplayed({ timeout: 20000 });
    await baseInput.clearValue();
    await baseInput.setValue(normalizedBaseUrl);

    const saveButton = await $('[data-testid="community-node-save-config"]');
    await saveButton.waitForClickable({ timeout: 20000 });
    await saveButton.scrollIntoView();
    await saveButton.click();

    nodeIndex = await waitForNodeIndex(
      normalizedBaseUrl,
      `Community node entry did not appear after saving config for ${normalizedBaseUrl}`,
    );
  }

  const authButton = await $(`[data-testid="community-node-authenticate-${nodeIndex}"]`);
  await authButton.waitForClickable({ timeout: 30000 });
  await authButton.scrollIntoView();
  await authButton.click();

  await browser.waitUntil(
    async () => {
      const currentSnapshot = await captureCommunityNodeSnapshot();
      const currentNodeIndex = findNodeIndex(currentSnapshot, normalizedBaseUrl);
      if (currentNodeIndex === undefined) {
        return false;
      }
      const node = currentSnapshot.nodes.find((entry) => entry.index === currentNodeIndex);
      return node?.hasToken === 'true' && Boolean(node.pubkey);
    },
    {
      timeout: 120000,
      interval: 1000,
      timeoutMsg: `Community node auth did not complete for ${normalizedBaseUrl}`,
    },
  );

  const finalSnapshot = await captureCommunityNodeSnapshot();
  const finalNodeIndex = findNodeIndex(finalSnapshot, normalizedBaseUrl);
  const finalNode =
    finalNodeIndex === undefined
      ? null
      : finalSnapshot.nodes.find((entry) => entry.index === finalNodeIndex) ?? null;

  if (finalNode?.hasToken !== 'true' || !finalNode.pubkey) {
    throw new Error(
      `Community node auth did not persist token for ${normalizedBaseUrl}: ${formatSnapshot(finalSnapshot)}`,
    );
  }
}
