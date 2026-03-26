import { expect, test } from '@playwright/test';

test('browser mock shell can run profile, private channel, live, and game flows', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 980 });
  await page.goto('/');

  await page.getByRole('tab', { name: 'Channels' }).click();
  await page.getByPlaceholder('core contributors').fill('Core Contributors');
  await page.getByRole('button', { name: 'Create Channel' }).click();
  await expect(page.getByRole('heading', { name: 'Core Contributors' })).toBeVisible();
  await page.getByRole('button', { name: 'Create Invite' }).click();
  await expect(page.getByText('Latest invite')).toBeVisible();

  await page
    .getByPlaceholder('paste private channel invite, friend grant, or friends+ share')
    .fill('invite-token');
  await page.getByRole('button', { name: 'Join Invite' }).click();
  await expect(page.getByRole('button', { name: 'kukuri:topic:demo' })).toBeVisible();

  await page.goto('/#/live');
  const liveTitle = page.getByLabel('Live Title');
  const liveDescription = page.getByLabel('Live Description');
  await liveTitle.fill('Launch Party');
  await expect(liveTitle).toHaveValue('Launch Party');
  await liveDescription.fill('watch along');
  await expect(liveDescription).toHaveValue('watch along');
  await page.getByRole('button', { name: 'Start Live' }).click();
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
  await page.getByPlaceholder('Top 8 Finals').fill('Grand Finals');
  await page.getByPlaceholder('match summary').fill('set one');
  await page.getByPlaceholder('Alice, Bob').fill('Alice, Bob');
  await page.getByRole('button', { name: 'Create Room' }).click();
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
