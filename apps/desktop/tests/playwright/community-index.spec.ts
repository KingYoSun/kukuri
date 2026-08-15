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

test('public and private indexing requests expose status and require private disclosure confirmation', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  await page.getByRole('button', { name: 'Request indexing for demo' }).click();
  let requestDialog = page.getByRole('dialog', { name: 'Request Community Node indexing' });
  await requestDialog.getByRole('button', { name: 'Submit request' }).click();
  await expect(requestDialog.getByText('The request is pending review.')).toBeVisible();
  await requestDialog.getByRole('button', { name: 'Close', exact: true }).click();

  await page.getByRole('button', { name: 'Channels' }).click();
  const channelDialog = page.getByRole('dialog', { name: 'Create / Join Private Channel' });
  await channelDialog.getByPlaceholder('Channel name').fill('Index Review');
  await channelDialog.getByRole('button', { name: 'Create Channel' }).click();
  await page.keyboard.press('Escape');

  await page.getByRole('button', { name: 'Open Index Review channel settings' }).click();
  const settingsDialog = page.getByRole('dialog', { name: 'Channel Settings' });
  await settingsDialog.getByRole('button', { name: 'Request indexing' }).click();
  requestDialog = page.getByRole('dialog', { name: 'Request Community Node indexing' });
  const submit = requestDialog.getByRole('button', { name: 'Submit request' });
  const confirmation = requestDialog.getByRole('checkbox', {
    name: /I agree to disclose this channel's read capability/,
  });
  await expect(submit).toBeDisabled();
  await confirmation.check();
  await expect(submit).toBeEnabled();
  await submit.click();
  await expect(requestDialog.getByText('The request is pending review.')).toBeVisible();
  await expect(confirmation).not.toBeChecked();
  await expect(submit).toBeDisabled();

  await confirmation.check();
  await expect(submit).toBeEnabled();
  await submit.click();
  await expect(requestDialog.getByText('The request is pending review.')).toBeVisible();
  await expect(confirmation).not.toBeChecked();
  await expect(submit).toBeDisabled();
});
