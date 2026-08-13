import { expect, test } from '@playwright/test';

test('topic search and cross-topic explore keep scope and reporting behavior visible', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  const topicIndex = page.getByTestId('community-index-topic');
  await expect(topicIndex).toBeVisible();
  await topicIndex.getByLabel('Search query').fill('browser mock peer');
  await topicIndex.getByRole('button', { name: 'Run' }).click();
  await expect(topicIndex.getByText('browser mock peer post')).toBeVisible();
  await expect(topicIndex.getByText(/not canonical post text/i)).toBeVisible();

  await topicIndex.getByRole('button', { name: 'Report' }).first().click();
  const reportDialog = page.getByRole('dialog', { name: 'Report content' });
  await expect(reportDialog).toContainText('api.kukuri.app');
  await expect(reportDialog).toContainText('Community index');
  await page.keyboard.press('Escape');

  await page.getByRole('tab', { name: 'Explore' }).click();
  await expect(page).toHaveURL(/#\/explore\?topic=/);
  const explore = page.getByTestId('community-index-explore');
  await explore.getByLabel('Search query').fill('iroh topic');
  await explore.getByRole('button', { name: 'Run' }).click();
  await expect(explore.getByText('iroh topic seed for builder preview')).toBeVisible();

  await explore.getByRole('tab', { name: 'Discover' }).click();
  await explore.getByRole('button', { name: 'Run' }).click();
  await expect(explore.getByRole('list', { name: 'Community Index results' })).toBeVisible();
});
