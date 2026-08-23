import { expect, test, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

// 既存フローは Live/Game タブなど WIP 面の表示を前提とするため developer mode を有効化する。
test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    window.localStorage.setItem(key, 'true');
  }, DEVELOPER_MODE_STORAGE_KEY);
});

async function openComposerDialog(page: Page) {
  const composer = page.getByPlaceholder(/Write a post|投稿を書く/);
  if (!(await composer.isVisible().catch(() => false))) {
    const englishTimeline = activeColumn(page, 'Timeline');
    if (await englishTimeline.isVisible().catch(() => false)) {
      await englishTimeline.getByRole('button', { name: /^Publish to / }).click();
    } else {
      await page.getByRole('button', { name: /^(Publish|投稿) to / }).last().click();
    }
  }
  await expect(composer).toBeVisible();
}

async function openControlCenter(page: Page) {
  const controlCenter = page.getByRole('complementary', { name: 'Control Center' });
  if (!(await controlCenter.isVisible().catch(() => false))) {
    await page.getByTestId('control-center-trigger').click();
  }
  await expect(controlCenter).toBeVisible();
  return controlCenter;
}

async function openSettings(page: Page) {
  const controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Settings', exact: true }).click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(settingsDialog).toBeVisible();
  return settingsDialog;
}

const TOPIC_ID_PREFIX = 'kukuri:topic:';
const LONG_PEER_ID = `12D3KooW${'a'.repeat(240)}`;
const LONG_PEER_TICKET = `${LONG_PEER_ID}@192.0.2.10:7777,2001:db8::10:7777`;
const LONG_ENDPOINT_DETAIL = `endpoint://${'b'.repeat(320)}@iroh-relay.kukuri.app:7842`;

function topicDisplayName(topicId: string): string {
  return topicId.startsWith(TOPIC_ID_PREFIX) ? topicId.slice(TOPIC_ID_PREFIX.length) : topicId;
}

function activeColumn(page: Page, title: string) {
  return page.getByRole('region', {
    name: new RegExp(`^${title} Column,.*Active,`),
  });
}

async function expectActiveTopic(page: Page, topic: string) {
  const controlCenter = await openControlCenter(page);
  const topicItem = controlCenter
    .getByRole('button', { name: topicDisplayName(topic), exact: true })
    .locator('xpath=ancestor::li[1]');
  await expect(topicItem).toHaveClass(/topic-item-active/);
  await controlCenter.getByRole('button', { name: 'Close Control Center' }).click();
}

test('browser mock wide shell keeps global navigation in the fixed Control Center', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.getByRole('complementary', { name: 'Primary navigation' })).toHaveCount(0);
  const trigger = page.getByTestId('control-center-trigger');
  await expect(trigger).toBeVisible();
  await expect(trigger).toHaveCSS('position', 'fixed');
  await trigger.click();
  await expect(page.getByRole('complementary', { name: 'Control Center' })).toBeVisible();
});

test('browser mock starts with one accessible Timeline Column without legacy workspace chrome', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  const layout = page.locator('.shell-phase1');
  const timelineColumn = page.getByRole('region', { name: /Timeline Column/ });

  await expect(layout).toHaveAttribute('data-workspace-layout', 'column');
  await expect(timelineColumn).toHaveCount(1);
  await expect(timelineColumn).toHaveAttribute('aria-current', 'true');
  await expect(timelineColumn).toHaveAttribute('data-span', '1');
  await expect(timelineColumn).toHaveAccessibleName(/Column 1 of 1/);
  await expect(timelineColumn).toHaveAccessibleName(/Pinned/);
  await expect(page.locator('main .shell-workspace-header-card')).toHaveCount(0);
  await expect(page.getByTestId('community-index-topic')).toHaveCount(0);
  await expect(page.getByRole('tablist', { name: 'Workspaces' })).toHaveCount(0);

  const columnHeader = timelineColumn.locator('.shell-column-header');
  const timelineViews = columnHeader.getByRole('tablist', { name: 'Timeline views' });
  const feedTab = timelineViews.getByRole('tab', { name: 'Feed' });
  const bookmarksTab = timelineViews.getByRole('tab', { name: 'Bookmarks' });
  const unpin = timelineColumn.getByRole('button', { name: 'Unpin Timeline' });
  await expect(feedTab.locator('svg')).toHaveCount(1);
  await expect(feedTab).toHaveText('');
  await expect(bookmarksTab.locator('svg')).toHaveCount(1);
  await expect(bookmarksTab).toHaveText('');
  await expect(timelineColumn.locator('.shell-column-body .shell-workspace-tabs')).toHaveCount(0);
  const [feedBox, bookmarksBox, unpinBox] = await Promise.all([
    feedTab.boundingBox(),
    bookmarksTab.boundingBox(),
    unpin.boundingBox(),
  ]);
  expect(feedBox).not.toBeNull();
  expect(bookmarksBox).not.toBeNull();
  expect(unpinBox).not.toBeNull();
  expect(feedBox!.width).toBe(unpinBox!.width);
  expect(feedBox!.height).toBe(unpinBox!.height);
  expect(bookmarksBox!.width).toBe(unpinBox!.width);
  expect(bookmarksBox!.height).toBe(unpinBox!.height);

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

  const controlCenter = await openControlCenter(page);
  await controlCenter.getByPlaceholder('demo').fill('kukuri:topic:browser');
  await controlCenter.getByRole('button', { name: 'Add', exact: true }).click();
  await controlCenter.getByRole('button', { name: /^browser$/ }).click();
  await expectActiveTopic(page, 'kukuri:topic:browser');

  await openComposerDialog(page);
  await page.getByPlaceholder('Write a post').fill('hello browser mock');
  await page.getByRole('button', { name: 'Publish', exact: true }).click();

  await expect(page.getByText('hello browser mock')).toBeVisible();

  await page.getByText('hello browser mock').click();
  const threadPane = activeColumn(page, 'Thread');
  await expect(threadPane).toBeVisible();
  await threadPane.getByRole('button', { name: 'ffffffffffff' }).first().click();
  await expect(activeColumn(page, 'Profile')).toBeVisible();

  const settingsDialog = await openSettings(page);
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
  const authorPane = activeColumn(page, 'Profile');
  await expect(authorPane).toBeVisible();

  await authorPane.getByRole('button', { name: 'Message' }).click();
  await expect(activeColumn(page, 'Conversation')).toBeVisible();
  await expect(page).toHaveURL(/#\/messages\?topic=.*peerPubkey=/);

  const workspace = page.locator('main[aria-label="Primary workspace"]');
  await workspace.getByRole('button', { name: 'browser peer' }).first().click();
  await expect(activeColumn(page, 'Profile')).toBeVisible();
  await expect(page).toHaveURL(/authorPubkey=/);

  await activeColumn(page, 'Profile').getByRole('button', { name: 'Close Profile' }).click();
  await expect(activeColumn(page, 'Conversation')).toBeVisible();
  await expect(activeColumn(page, 'Conversation')).toBeVisible();
  await expect(page).toHaveURL(/#\/messages\?topic=.*peerPubkey=/);
});

test('browser mock shell persists appearance theme changes across reloads', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

  const settingsDialog = await openSettings(page);
  await settingsDialog.getByTestId('settings-section-appearance').click();
  await settingsDialog.getByRole('radio', { name: /Light/i }).click();

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

  await page.reload();

  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
});

test('desktop Columns use kind spans, keyboard reorder, drag reorder, and persisted layout', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  let controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Add Live Column' }).click();
  const liveColumn = page.getByRole('region', { name: /^Live Column/ });
  await expect(liveColumn).toHaveAttribute('data-span', '2');
  expect(Math.round((await liveColumn.boundingBox())!.width)).toBe(896);
  const wideStreamTracks = await liveColumn.locator('.shell-stream-layout').evaluate(
    (element) => getComputedStyle(element).gridTemplateColumns
  );
  expect(wideStreamTracks.split(' ').length).toBeGreaterThan(1);

  controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Add Metaverse Column' }).click();
  let metaverseColumn = page.getByRole('region', { name: /^Metaverse Column/ });
  await expect(metaverseColumn).toHaveAttribute('data-span', '3');
  expect(Math.round((await metaverseColumn.boundingBox())!.width)).toBe(1352);

  await metaverseColumn.getByRole('button', { name: 'Open Metaverse menu' }).click();
  const menu = page.getByRole('menu', { name: 'Metaverse actions' });
  await expect(menu).toBeVisible();
  await menu.getByRole('menuitemradio', { name: '4 spans' }).click();
  await expect(metaverseColumn).toHaveAttribute('data-span', '4');
  expect(Math.round((await metaverseColumn.boundingBox())!.width)).toBe(1808);

  await metaverseColumn.getByRole('button', { name: 'Open Metaverse menu' }).click();
  await page.getByRole('menuitem', { name: 'Move Metaverse left' }).click();
  await expect(metaverseColumn).toHaveAccessibleName(/Column 2 of 3/);

  const canvas = page.locator('.shell-column-canvas');
  await canvas.evaluate((element) => { element.scrollLeft = 0; });
  const metaverseGrip = metaverseColumn.getByRole('button', {
    name: 'Move Metaverse Column',
  });
  await metaverseGrip.dragTo(canvas, {
    targetPosition: { x: 12, y: 160 },
  });
  await expect(metaverseColumn).toHaveAccessibleName(/Column 1 of 3/);
  await expect(liveColumn).toHaveAccessibleName(/Column 3 of 3/);

  await page.reload();
  metaverseColumn = page.getByRole('region', { name: /^Metaverse Column/ });
  await expect(page.getByRole('region', { name: /^Live Column/ })).toHaveAccessibleName(/Column 3 of 3/);
  await expect(metaverseColumn).toHaveAttribute('data-span', '4');
  await expect(metaverseColumn).toHaveAccessibleName(/Column 1 of 3/);
  await expect(page).not.toHaveURL(/span|layout|column/i);
});

test('mobile Columns page one viewport at a time with snap, direct jump, and history back', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  await openComposerDialog(page);
  await page.getByPlaceholder('Write a post').fill('mobile paging thread');
  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await page.getByText('mobile paging thread').click();
  const thread = activeColumn(page, 'Thread');
  await expect(thread).toBeVisible();
  await thread.getByRole('button', { name: 'ffffffffffff' }).first().click();
  await expect(activeColumn(page, 'Profile')).toBeVisible();

  const canvas = page.locator('.shell-column-canvas');
  const geometry = await page.evaluate(() => {
    const root = document.querySelector<HTMLElement>('.shell-column-canvas')!;
    return {
      canvasWidth: root.clientWidth,
      snapType: getComputedStyle(root).scrollSnapType,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      widths: Array.from(root.querySelectorAll<HTMLElement>('[data-column-id]')).map(
        (column) => Math.round(column.getBoundingClientRect().width)
      ),
    };
  });
  expect(geometry.snapType).toContain('mandatory');
  expect(geometry.overflow).toBeLessThanOrEqual(0);
  expect(geometry.widths.every((width) => Math.abs(width - geometry.canvasWidth) <= 1)).toBe(true);
  const headerTargets = await activeColumn(page, 'Profile')
    .locator('.shell-column-header-actions button:visible')
    .evaluateAll((buttons) => buttons.map((button) => {
      const rect = button.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    }));
  expect(headerTargets.every((box) => box.width >= 44 && box.height >= 44)).toBe(true);

  await page.goBack();
  await expect(activeColumn(page, 'Thread')).toBeVisible();
  await page.goBack();
  await expect(activeColumn(page, 'Timeline')).toBeVisible();

  await page.getByRole('button', { name: 'Go to Column 3 of 3' }).click();
  await expect(activeColumn(page, 'Profile')).toBeVisible();
  await page.getByRole('button', { name: 'Go to Column 1 of 3' }).click();
  await expect(activeColumn(page, 'Timeline')).toBeVisible();
  await expect(page.getByText('1 / 3')).toBeVisible();
  await canvas.evaluate((element) => {
    element.scrollLeft = element.clientWidth;
    element.dispatchEvent(new Event('scroll'));
  });
  await expect(activeColumn(page, 'Thread')).toBeVisible();
  await page.getByRole('button', { name: 'Go to Column 1 of 3' }).click();
  await expect(activeColumn(page, 'Timeline')).toBeVisible();
  await expect(canvas).toBeVisible();
});

test('mobile restart restores text Draft and active Column focus without unsafe fields', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');
  await openComposerDialog(page);
  const composer = page.getByPlaceholder('Write a post');
  await composer.fill('restart-safe mobile Draft');
  const composerLayout = await page.evaluate(() => ({
    controlCenterDisplay: getComputedStyle(
      document.querySelector<HTMLElement>('.shell-control-center-trigger')!
    ).display,
    footerRight: document.querySelector<HTMLElement>('.shell-column-footer')!
      .getBoundingClientRect().right,
    viewportWidth: document.documentElement.clientWidth,
  }));
  expect(composerLayout.controlCenterDisplay).toBe('none');
  expect(composerLayout.footerRight).toBeLessThanOrEqual(composerLayout.viewportWidth);
  await page.waitForTimeout(300);

  const payload = await page.evaluate(() => localStorage.getItem('kukuri:column-drafts:v1'));
  expect(payload).toContain('restart-safe mobile Draft');
  expect(payload).not.toContain('mediaItems');
  expect(payload).not.toContain('pending');
  expect(payload).not.toContain('error');

  await page.reload();
  await expect(page.getByPlaceholder('Write a post')).toHaveValue('restart-safe mobile Draft');
  await expect(activeColumn(page, 'Timeline')).toBeFocused();
});

test('browser mock shell persists language changes across reloads', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await openComposerDialog(page);
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();
  await page.keyboard.press('Escape');

  const settingsDialog = await openSettings(page);
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

  await page.goto('/#/game');
  await expect(page.getByRole('heading', { name: 'メタバースルーム' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'メタバースルームを作成' })).toBeVisible();
});

test('browser mock settings drawer keeps the close button clear of content and captures wheel scrolling', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  const settingsDialog = await openSettings(page);
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

  let controlCenter = await openControlCenter(page);
  await controlCenter.getByPlaceholder('demo').fill('kukuri:topic:narrow');
  await controlCenter.getByRole('button', { name: 'Add', exact: true }).click();

  controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: /^demo$/ }).click();
  await expectActiveTopic(page, 'kukuri:topic:demo');

  await openComposerDialog(page);
  await page.getByPlaceholder('Write a post').fill('narrow browser mock');
  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByText('narrow browser mock')).toBeVisible();

  await page.getByText('narrow browser mock').click();
  const threadColumn = activeColumn(page, 'Thread');
  await expect(threadColumn).toBeVisible();

  await threadColumn.getByRole('button', { name: 'ffffffffffff' }).first().click();
  await expect(activeColumn(page, 'Profile')).toBeVisible();

  await page.goto('/');
  const settingsDialog = await openSettings(page);
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
  await page.goto('/#/profile?topic=kukuri%3Atopic%3Ademo');
  await expect(page.getByRole('button', { name: 'Edit Profile' })).toBeVisible();

  const noOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth
  );
  expect(noOverflow).toBeTruthy();
});
