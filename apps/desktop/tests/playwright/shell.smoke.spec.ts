import { expect, test, type Page } from '@playwright/test';

async function openComposerDialog(page: Page) {
  await page.getByTestId('shell-fab').click();
  await expect(page.getByRole('dialog')).toBeVisible();
}

async function expectActiveTopic(page: Page, topic: string) {
  const navRail = page.getByRole('complementary', { name: 'Primary navigation' });
  const topicItem = navRail
    .getByRole('button', { name: topic, exact: true })
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

test('browser mock shell can switch topics, publish, open thread, open author, and update discovery from settings', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expectActiveTopic(page, 'kukuri:topic:demo');

  await page.getByPlaceholder('kukuri:topic:demo').fill('kukuri:topic:browser');
  await page.getByRole('button', { name: 'Add' }).click();
  await page.getByRole('button', { name: /^kukuri:topic:browser$/ }).click();
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

test('browser mock narrow shell keeps nav, context, and settings flows reachable without overflow', async ({
  page,
}) => {
  await page.setViewportSize({ width: 700, height: 980 });
  await page.goto('/');

  await page.getByTestId('shell-nav-trigger').click();
  await page.getByPlaceholder('kukuri:topic:demo').fill('kukuri:topic:narrow');
  await page.getByRole('button', { name: 'Add' }).click();

  await page.getByTestId('shell-nav-trigger').click();
  await page.getByRole('button', { name: /^kukuri:topic:demo$/ }).click();
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
  await page.getByRole('tab', { name: 'Profile' }).click();
  await expect(page.getByRole('button', { name: 'Edit Profile' })).toBeVisible();

  const noOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth
  );
  expect(noOverflow).toBeTruthy();
});
