import { expect, test } from '@playwright/test';

async function openControlCenter(page: import('@playwright/test').Page) {
  const controlCenter = page.getByRole('complementary', { name: 'Control Center' });
  if (!(await controlCenter.isVisible().catch(() => false))) {
    await page.getByTestId('control-center-trigger').click();
  }
  await expect(controlCenter).toBeVisible();
  return controlCenter;
}

test('Timeline keeps Community Index out of the primary surface and Explore keeps reporting behavior visible', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  const topicIndex = page.getByTestId('community-index-topic');
  await expect(topicIndex).toHaveCount(0);

  const controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Add Explore Column' }).click();
  await expect(page).toHaveURL(/#\/explore\?topic=/);
  const explore = page.getByTestId('community-index-explore');
  await explore.getByLabel('Search query').fill('iroh topic');
  await explore.getByRole('button', { name: 'Run' }).click();
  await expect(explore.getByText('iroh topic seed for builder preview')).toBeVisible();

  await explore.getByRole('tab', { name: 'Discover' }).click();
  await explore.getByRole('button', { name: 'Run' }).click();
  await expect(explore.getByRole('list', { name: 'Community Index results' })).toBeVisible();

  await explore.getByRole('tab', { name: 'Recommendations' }).click();
  await explore.getByRole('button', { name: 'Run' }).click();
  await expect(explore.getByRole('list', { name: 'Community Index results' })).toBeVisible();
  await explore.getByRole('button', { name: 'Report' }).first().click();
  const reportDialog = page.getByRole('dialog', { name: 'Report content' });
  await expect(reportDialog).toContainText('Recommendation');
  await expect(reportDialog).toContainText('api.kukuri.app');
  await expect(reportDialog.getByRole('button', { name: 'Send report' })).toBeVisible();
});

test('public and private indexing requests expose status and require private disclosure confirmation', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

  let controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Request indexing for demo' }).click();
  let requestDialog = page.getByRole('dialog', { name: 'Request Community Node indexing' });
  await requestDialog.getByRole('button', { name: 'Submit request' }).click();
  await expect(requestDialog.getByText('The request is pending review.')).toBeVisible();
  await requestDialog.getByRole('button', { name: 'Close', exact: true }).click();

  controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Create or join channel' }).click();
  const channelDialog = page.getByRole('dialog', { name: 'Create / Join Private Channel' });
  await channelDialog.getByPlaceholder('Channel name').fill('Index Review');
  await channelDialog.getByRole('button', { name: 'Create Channel' }).click();
  await page.keyboard.press('Escape');

  controlCenter = await openControlCenter(page);
  await controlCenter
    .getByRole('button', { name: 'Open Index Review channel settings' })
    .click();
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

test('advanced Community Node settings persist manual and automatic index preferences', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ademo');

  let controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Settings', exact: true }).click();
  let settings = page.getByRole('dialog', { name: 'Settings' });
  await settings.getByTestId('settings-section-community-node').click();
  let selector = settings.getByRole('combobox', { name: 'Community Index query node' });
  const selectorBox = (await selector.boundingBox())!;
  expect(selectorBox.height).toBeGreaterThanOrEqual(44);
  await selector.selectOption('https://api.kukuri.app');
  await expect.poll(() => page.evaluate(() =>
    localStorage.getItem('kukuri:community-index-node-preference:v1')
  )).toContain('manual');

  await page.reload();
  controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Settings', exact: true }).click();
  settings = page.getByRole('dialog', { name: 'Settings' });
  await settings.getByTestId('settings-section-community-node').click();
  selector = settings.getByRole('combobox', { name: 'Community Index query node' });
  await expect(selector).toHaveValue('https://api.kukuri.app');
  await selector.selectOption('auto');
  await expect.poll(() => page.evaluate(() =>
    localStorage.getItem('kukuri:community-index-node-preference:v1')
  )).toContain('auto');
});
