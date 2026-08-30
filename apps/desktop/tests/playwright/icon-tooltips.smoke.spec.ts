import { expect, test, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    window.localStorage.setItem(key, 'true');
  }, DEVELOPER_MODE_STORAGE_KEY);
});

function activeColumn(page: Page, title: string) {
  return page.getByRole('region', {
    name: new RegExp(`^${title} Column,.*Active,`),
  });
}

test('icon-only controls expose the same localized action on hover and focus', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 760 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');

  const timeline = activeColumn(page, 'Timeline');
  const feed = timeline.getByRole('tab', { name: 'Feed' });
  await feed.hover();
  await expect(page.getByRole('tooltip')).toHaveText('Feed');

  await page.keyboard.press('Escape');
  await expect(page.getByRole('tooltip')).toHaveCount(0);

  await feed.focus();
  await expect(feed).toBeFocused();
  await expect(page.getByRole('tooltip')).toHaveText('Feed');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('tooltip')).toHaveCount(0);

  await page.goto('/#/notifications?topic=kukuri%3Atopic%3Ageneral');
  const notifications = activeColumn(page, 'Notifications');
  const header = notifications.locator('.shell-column-header');
  await expect(header).toContainText(/items.*unread/);
  await expect(header.getByRole('button', { name: 'Refresh' })).toBeVisible();
  await expect(notifications.locator('.shell-column-body .shell-workspace-header')).toHaveCount(0);
});
