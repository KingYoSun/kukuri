import { expect, test, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

// 既存フローは Live/Game タブなど WIP 面の表示を前提とするため developer mode を有効化する。
test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    window.localStorage.setItem(key, 'true');
  }, DEVELOPER_MODE_STORAGE_KEY);
});

async function openComposerDialog(page: Page) {
  await page.getByTestId('shell-fab').click();
  await expect(page.getByRole('dialog')).toBeVisible();
}

const TOPIC_ID_PREFIX = 'kukuri:topic:';
const LONG_PEER_ID = `12D3KooW${'a'.repeat(240)}`;
const LONG_PEER_TICKET = `${LONG_PEER_ID}@192.0.2.10:7777,2001:db8::10:7777`;
const LONG_ENDPOINT_DETAIL = `endpoint://${'b'.repeat(320)}@iroh-relay.kukuri.app:7842`;

function topicDisplayName(topicId: string): string {
  return topicId.startsWith(TOPIC_ID_PREFIX) ? topicId.slice(TOPIC_ID_PREFIX.length) : topicId;
}

async function expectActiveTopic(page: Page, topic: string) {
  const navRail = page.getByRole('complementary', { name: 'Primary navigation' });
  const topicItem = navRail
    .getByRole('button', { name: topicDisplayName(topic), exact: true })
    .locator('xpath=ancestor::li[1]');
  await expect(topicItem).toHaveClass(/topic-item-active/);
}

test('browser mock wide shell keeps navigation rail beside the workspace', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.getByTestId('shell-nav-trigger')).toHaveCount(0);

  const navRail = page.getByRole('complementary', { name: 'Primary navigation' });
  const workspace = page.locator('main[aria-label="Primary workspace"]');
  const navBox = await navRail.boundingBox();
  const workspaceBox = await workspace.boundingBox();

  expect(navBox).not.toBeNull();
  expect(workspaceBox).not.toBeNull();
  expect(navBox!.x + navBox!.width).toBeLessThan(workspaceBox!.x);
});

test('browser mock starts with one accessible Timeline Column without legacy workspace chrome', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  const layout = page.locator('.shell-layout');
  const timelineColumn = page.getByRole('region', { name: /Timeline Column/ });

  await expect(layout).toHaveAttribute('data-workspace-layout', 'column');
  await expect(timelineColumn).toHaveCount(1);
  await expect(timelineColumn).toHaveAttribute('aria-current', 'true');
  await expect(timelineColumn).toHaveAttribute('data-span', '1');
  await expect(timelineColumn).toHaveAccessibleName(/Column 1 of 1/);
  await expect(timelineColumn).toHaveAccessibleName(/Pinned/);
  await expect(page.locator('main .shell-workspace-header-card')).toHaveCount(0);
  await expect(page.getByTestId('community-index-topic')).toHaveCount(0);
  await expect
    .poll(() =>
      page
        .getByRole('tablist', { name: 'Workspaces' })
        .evaluate((element) => getComputedStyle(element).gridTemplateColumns.split(' ').length)
    )
    .toBe(2);
  const workspaceTabRows = await page
    .getByRole('tablist', { name: 'Workspaces' })
    .getByRole('tab')
    .evaluateAll((tabs) => new Set(tabs.map((tab) => tab.getBoundingClientRect().y)).size);
  expect(workspaceTabRows).toBe(3);

  const unpin = timelineColumn.getByRole('button', { name: 'Unpin Timeline' });
  await unpin.click();
  await expect(timelineColumn).toHaveAttribute('data-transient', 'true');
  await expect(timelineColumn).toHaveAccessibleName(/Temporary/);

  const overflow = await page.evaluate(() => ({
    canvasOwnsHorizontalOverflow:
      getComputedStyle(document.querySelector('.shell-column-canvas')!).overflowX === 'auto',
    documentOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
  }));
  expect(overflow.canvasOwnsHorizontalOverflow).toBe(true);
  expect(overflow.documentOverflow).toBeLessThanOrEqual(0);
});

test('browser mock shell can switch topics, publish, open thread, open author, and update discovery from settings', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expectActiveTopic(page, 'kukuri:topic:demo');

  await page.getByPlaceholder('demo').fill('kukuri:topic:browser');
  await page.getByRole('button', { name: 'Add' }).click();
  await page.getByRole('button', { name: /^browser$/ }).click();
  await expectActiveTopic(page, 'kukuri:topic:browser');

  await openComposerDialog(page);
  await page.getByPlaceholder('Write a post').fill('hello browser mock');
  await page.getByRole('button', { name: 'Publish' }).click();

  await expect(page.getByText('hello browser mock')).toBeVisible();

  await page.getByText('hello browser mock').click();
  const threadPane = page.getByRole('complementary', { name: 'Thread' });
  await expect(threadPane).toBeVisible();
  await page
    .getByRole('complementary', { name: 'Thread' })
    .getByRole('button', { name: 'ffffffffffff' })
    .first()
    .click();
  await expect(page.getByRole('complementary', { name: 'Author' })).toBeVisible();

  await page.getByTestId('shell-settings-trigger').click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(settingsDialog).toBeVisible();
  await settingsDialog.getByTestId('settings-section-discovery').click();
  await settingsDialog.getByPlaceholder('node_id or node_id@host:port').fill('seed-peer-1');
  await settingsDialog.getByRole('button', { name: 'Save Seeds' }).click();
  await expect(settingsDialog.getByRole('textbox', { name: 'Seed Peers' })).toHaveValue('seed-peer-1');

  await settingsDialog.getByRole('textbox', { name: 'Seed Peers' }).fill('seed-peer-1\nseed-peer-2');
  await settingsDialog.getByRole('button', { name: 'Reset' }).click();
  await expect(settingsDialog.getByRole('textbox', { name: 'Seed Peers' })).toHaveValue('seed-peer-1');

  await settingsDialog.getByTestId('settings-section-community-node').click();
  await expect(
    settingsDialog.getByRole('checkbox', { name: 'Auto-approve consent for this node' })
  ).toBeChecked();
  await expect(settingsDialog.getByText('active on current session', { exact: true })).toBeVisible();
  await expect(settingsDialog.getByText('connectivity urls active on current session')).toBeVisible();

  await settingsDialog.locator('button').filter({ hasText: /^Add Node$/ }).click();
  await settingsDialog
    .getByPlaceholder('https://community.example.com')
    .last()
    .fill('https://community.example.com');
  await settingsDialog.getByRole('button', { name: 'Save Nodes', exact: true }).click();
  await expect(
    settingsDialog.getByRole('heading', { name: 'https://community.example.com' })
  ).toBeVisible();

  await settingsDialog.getByRole('button', { name: 'Refresh' }).first().click();
  await expect(settingsDialog.getByRole('heading', { name: 'https://api.kukuri.app' })).toBeVisible();
});

test('browser mock shell can open an author from messages without leaving the dm workspace', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await page.getByRole('button', { name: 'browser peer' }).first().click();
  const authorPane = page.getByRole('complementary', { name: 'Author' });
  await expect(authorPane).toBeVisible();

  await authorPane.getByRole('button', { name: 'Message' }).click();
  await expect(page.getByRole('tab', { name: 'Messages' })).toHaveAttribute('aria-selected', 'true');
  await expect(page).toHaveURL(/#\/messages\?topic=.*peerPubkey=/);

  const workspace = page.locator('main[aria-label="Primary workspace"]');
  await workspace.getByRole('button', { name: 'browser peer' }).first().click();
  await expect(page.getByRole('complementary', { name: 'Author' })).toBeVisible();
  await expect(page).toHaveURL(/authorPubkey=/);

  await page.getByRole('button', { name: 'Close Author' }).click();
  await expect(page.getByRole('complementary', { name: 'Author' })).toHaveCount(0);
  await expect(page.getByRole('tab', { name: 'Messages' })).toHaveAttribute('aria-selected', 'true');
  await expect(page).toHaveURL(/#\/messages\?topic=.*peerPubkey=/);
});

test('browser mock shell persists appearance theme changes across reloads', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

  await page.getByTestId('shell-settings-trigger').click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await settingsDialog.getByTestId('settings-section-appearance').click();
  await settingsDialog.getByRole('radio', { name: /Light/i }).click();

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

  await page.reload();

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
});

test('browser mock shell persists language changes across reloads', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await openComposerDialog(page);
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();
  await page.keyboard.press('Escape');

  await page.getByTestId('shell-settings-trigger').click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await settingsDialog.getByTestId('settings-section-appearance').click();
  await settingsDialog.getByLabel('Language').selectOption('ja');

  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await page.keyboard.press('Escape');
  await expect(settingsDialog).toBeHidden();
  await openComposerDialog(page);
  await expect(page.getByPlaceholder('投稿を書く')).toBeVisible();
  await expect(page.getByRole('button', { name: '投稿' })).toBeVisible();
  await page.keyboard.press('Escape');

  await page.reload();

  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await openComposerDialog(page);
  await expect(page.getByPlaceholder('投稿を書く')).toBeVisible();
  await page.keyboard.press('Escape');

  await page.getByRole('tab', { name: 'ゲーム' }).click();
  await expect(page.getByRole('heading', { name: 'メタバースルーム' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'メタバースルームを作成' })).toBeVisible();
});

test('browser mock settings drawer keeps the close button clear of content and captures wheel scrolling', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await page.getByTestId('shell-settings-trigger').click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(settingsDialog).toBeVisible();
  await settingsDialog.getByTestId('settings-section-connectivity').click();

  const closeButton = settingsDialog.getByRole('button', { name: 'Close settings' });
  const syncStatusCard = settingsDialog
    .getByRole('heading', { name: 'Sync Status' })
    .locator('xpath=ancestor::section[1]');
  const scrollContainer = settingsDialog.locator('.shell-settings-content');

  await expect.poll(() =>
    scrollContainer.evaluate((element) => element.scrollHeight > element.clientHeight)
  ).toBeTruthy();

  const closeBox = await closeButton.boundingBox();
  const syncStatusBox = await syncStatusCard.boundingBox();

  expect(closeBox).not.toBeNull();
  expect(syncStatusBox).not.toBeNull();
  expect(closeBox!.y + closeBox!.height).toBeLessThanOrEqual(syncStatusBox!.y);

  const scrollBox = await scrollContainer.boundingBox();
  expect(scrollBox).not.toBeNull();

  await page.mouse.move(
    scrollBox!.x + scrollBox!.width / 2,
    scrollBox!.y + Math.min(scrollBox!.height / 2, 220)
  );
  await page.mouse.wheel(0, 960);

  await expect.poll(() => scrollContainer.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
});

test('browser mock connectivity settings keep long identifiers within the content width', async ({
  page,
}) => {
  await page.addInitScript(
    ({ endpointDetail, peerId, peerTicket }) => {
      let desktopApi = window.__KUKURI_DESKTOP__;

      Object.defineProperty(window, '__KUKURI_DESKTOP__', {
        configurable: true,
        get: () => desktopApi,
        set: (api: typeof desktopApi) => {
          desktopApi = api;
          if (!api) {
            return;
          }

          const getSyncStatus = api.getSyncStatus.bind(api);
          api.getSyncStatus = async () => {
            const status = await getSyncStatus();
            return {
              ...status,
              configured_peers: [peerId],
              status_detail: endpointDetail,
              discovery: {
                ...status.discovery,
                connected_peer_ids: [peerId],
                local_endpoint_id: peerId,
              },
              topic_diagnostics: status.topic_diagnostics.map((diagnostic) => ({
                ...diagnostic,
                connected_peers: [peerId],
                configured_peer_ids: [peerId],
                docs_assist_peer_ids: [peerId],
                status_detail: endpointDetail,
              })),
            };
          };
          api.getLocalPeerTicket = async () => peerTicket;
        },
      });
    },
    {
      endpointDetail: LONG_ENDPOINT_DETAIL,
      peerId: LONG_PEER_ID,
      peerTicket: LONG_PEER_TICKET,
    }
  );

  await page.setViewportSize({ width: 1024, height: 870 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo&settings=connectivity');

  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  const scrollContainer = settingsDialog.locator('.shell-settings-content');
  const localPeerTicket = settingsDialog.getByRole('textbox', { name: 'Your Ticket' });
  await expect(settingsDialog).toBeVisible();
  await expect(localPeerTicket).toHaveValue(LONG_PEER_TICKET);
  await expect(settingsDialog.getByText(LONG_ENDPOINT_DETAIL).first()).toBeVisible();
  await expect(settingsDialog.getByText(LONG_PEER_ID).first()).toBeVisible();

  for (const width of [1024, 1280, 1440, 700]) {
    await page.setViewportSize({ width, height: 870 });

    const layout = await scrollContainer.evaluate((element) => {
      const contentRect = element.getBoundingClientRect();
      const contentRight = contentRect.left + element.clientWidth;
      const overflowingPanels = Array.from(element.querySelectorAll<HTMLElement>('.panel')).filter(
        (panel) => panel.getBoundingClientRect().right > contentRight + 0.5
      );

      return {
        contentFits: element.scrollWidth <= element.clientWidth,
        horizontalOverflowPolicy: getComputedStyle(element).overflowX,
        overflowingPanelCount: overflowingPanels.length,
      };
    });
    const ticketFits = await localPeerTicket.evaluate(
      (element) => element.scrollWidth <= element.clientWidth
    );

    expect(layout, `viewport width: ${width}`).toEqual({
      contentFits: true,
      horizontalOverflowPolicy: 'hidden',
      overflowingPanelCount: 0,
    });
    expect(ticketFits, `peer ticket at viewport width: ${width}`).toBeTruthy();
    await expect(settingsDialog.getByRole('button', { name: 'Import Peer' })).toBeVisible();
  }
});

test('browser mock narrow shell keeps nav, context, and settings flows reachable without overflow', async ({
  page,
}) => {
  await page.setViewportSize({ width: 700, height: 980 });
  await page.goto('/');

  await page.getByTestId('shell-nav-trigger').click();
  await page.getByPlaceholder('demo').fill('kukuri:topic:narrow');
  await page.getByRole('button', { name: 'Add' }).click();

  await page.getByTestId('shell-nav-trigger').click();
  await page.getByRole('button', { name: /^demo$/ }).click();
  await expectActiveTopic(page, 'kukuri:topic:demo');

  await openComposerDialog(page);
  await page.getByPlaceholder('Write a post').fill('narrow browser mock');
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page.getByText('narrow browser mock')).toBeVisible();

  await page.getByText('narrow browser mock').click();
  await expect(page.getByRole('complementary', { name: 'Thread' })).toBeVisible();

  await page
    .getByRole('complementary', { name: 'Thread' })
    .getByRole('button', { name: 'ffffffffffff' })
    .first()
    .click();
  await expect(page.getByRole('complementary', { name: 'Author' })).toBeVisible();

  await page.goto('/');
  await page.getByTestId('shell-nav-trigger').click();
  await page.getByTestId('shell-settings-trigger').click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await settingsDialog.getByTestId('settings-section-connectivity').click();
  await settingsDialog.getByPlaceholder('nodeid@127.0.0.1:7777').fill('peer-b@127.0.0.1:8888');
  await settingsDialog.getByRole('button', { name: 'Import Peer' }).click();
  await expect(settingsDialog.getByPlaceholder('nodeid@127.0.0.1:7777')).toHaveValue('');

  await settingsDialog.getByTestId('settings-section-community-node').click();
  await settingsDialog.locator('button').filter({ hasText: /^Add Node$/ }).click();
  await settingsDialog
    .getByPlaceholder('https://community.example.com')
    .last()
    .fill('https://community.example.com');
  await settingsDialog.getByRole('button', { name: 'Save Nodes', exact: true }).click();
  await expect(
    settingsDialog.getByRole('heading', { name: 'https://community.example.com' })
  ).toBeVisible();

  const settingsNoOverflow = await settingsDialog.evaluate(
    (element) => element.scrollWidth <= element.clientWidth
  );
  expect(settingsNoOverflow).toBeTruthy();

  await page.keyboard.press('Escape');
  await page
    .getByRole('complementary', { name: 'Primary navigation' })
    .getByLabel('Close navigation')
    .click();
  await page.goto('/#/profile?topic=kukuri%3Atopic%3Ademo');
  await expect(page.getByRole('button', { name: 'Edit Profile' })).toBeVisible();

  const noOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth
  );
  expect(noOverflow).toBeTruthy();
});
