import { expect, test, type Page } from '@playwright/test';

async function openChannelManager(page: Page) {
  const dialog = page.getByRole('dialog', { name: 'Create / Join Private Channel' });
  if (await dialog.isVisible().catch(() => false)) {
    return dialog;
  }
  await page.getByRole('button', { name: 'Channels' }).click();
  await expect(dialog).toBeVisible();
  return dialog;
}

async function openChannelSettings(page: Page, channelLabel: string) {
  await page.getByRole('button', { name: `Open ${channelLabel} channel settings` }).click();
  const dialog = page.getByRole('dialog', { name: 'Channel Settings' });
  await expect(dialog).toBeVisible();
  return dialog;
}

async function openFloatingActionDialog(page: Page) {
  await page.getByTestId('shell-fab').click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  return dialog;
}

test('browser mock shell can run profile, private channel, live, and game flows', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 980 });
  await page.goto('/');

  const channelDialog = await openChannelManager(page);
  await channelDialog.getByPlaceholder('Channel name').fill('Core Contributors');
  await channelDialog.getByRole('button', { name: 'Create Channel' }).click();
  await expect(page).toHaveURL(/#\/timeline\?topic=.*&channel=channel-1/);
  await page.keyboard.press('Escape');

  const channelSettingsDialog = await openChannelSettings(page, 'Core Contributors');
  await channelSettingsDialog.getByRole('button', { name: 'Create share link' }).click();
  await expect(channelSettingsDialog.getByText('Copy share link')).toBeVisible();
  await expect(channelSettingsDialog.getByRole('button', { name: 'Copy link' })).toBeVisible();
  await page.keyboard.press('Escape');

  const joinDialog = await openChannelManager(page);
  await joinDialog
    .getByPlaceholder('paste private channel invite, friend grant, or friends+ share')
    .fill('invite-token');
  await joinDialog.getByRole('button', { name: 'Join' }).click();
  await expect(page).toHaveURL(
    /#\/timeline\?topic=kukuri%3Atopic%3Ademo&channel=channel-imported/
  );
  await page.keyboard.press('Escape');

  await page.goto('/#/live');
  const liveDialog = await openFloatingActionDialog(page);
  const liveTitle = liveDialog.getByLabel('Live Title');
  const liveDescription = liveDialog.getByLabel('Live Description');
  await liveTitle.fill('Launch Party');
  await expect(liveTitle).toHaveValue('Launch Party');
  await liveDescription.fill('watch along');
  await expect(liveDescription).toHaveValue('watch along');
  await liveDialog.getByRole('button', { name: 'Start Live' }).click();
  const liveCard = page.locator('article.post-card').filter({ has: page.getByText('Launch Party') });
  await expect(liveCard).toBeVisible();
  await page
    .locator('article.post-card')
    .filter({ has: page.getByText('Launch Party') })
    .getByRole('button', { name: 'Join', exact: true })
    .click();
  await expect(page.getByText('viewers: 1')).toBeVisible();
  await page
    .locator('article.post-card')
    .filter({ has: page.getByText('Launch Party') })
    .getByRole('button', { name: 'End', exact: true })
    .click();
  await expect(
    page
      .locator('article.post-card')
      .filter({ has: page.getByText('Launch Party') })
      .getByText('Ended', { exact: true })
  ).toBeVisible();

  await page.goto('/#/game');
  const gameDialog = await openFloatingActionDialog(page);
  await gameDialog.getByPlaceholder('Top 8 Finals').fill('Grand Finals');
  await gameDialog.getByPlaceholder('match summary').fill('set one');
  await gameDialog.getByPlaceholder('Alice, Bob').fill('Alice, Bob');
  await gameDialog.getByRole('button', { name: 'Create Room' }).click();
  await expect(page.getByText('Grand Finals')).toBeVisible();
  await page.getByLabel(/game-.*-status/).selectOption('Running');
  await page.getByLabel(/game-.*-phase/).fill('Round 3');
  await page.getByLabel(/game-.*-Alice-score/).fill('2');
  await page.getByRole('button', { name: 'Save Room' }).click();
  await expect(page.getByText('phase: Round 3')).toBeVisible();

  await page.goto('/#/profile?profileMode=edit');
  await page.getByPlaceholder('Visible label').fill('Browser Author');
  await page.getByRole('button', { name: 'Save Profile' }).click();
  await expect(page.getByText('Browser Author')).toBeVisible();
});
