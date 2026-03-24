import { expect, test } from '@playwright/test';

test('browser mock shell can run profile, private channel, live, and game flows', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 980 });
  await page.goto('/');

  await page.getByPlaceholder('core contributors').fill('Core Contributors');
  await page.getByRole('button', { name: 'Create Channel' }).click();
  await expect(page.getByLabel('Compose Target')).toHaveValue('channel:channel-1');
  await expect(page.getByText(/Posting to: Core Contributors/i)).toBeVisible();
  await page.getByRole('button', { name: 'Create Invite' }).click();
  await expect(page.getByText('Latest invite')).toBeVisible();

  await page
    .getByPlaceholder('paste private channel invite, friend grant, or friends+ share')
    .fill('invite-token');
  await page.getByRole('button', { name: 'Join Invite' }).click();
  await expect(page.getByRole('button', { name: 'kukuri:topic:demo' })).toBeVisible();

  await page.getByPlaceholder('Friday stream').fill('Launch Party');
  await page.getByPlaceholder('short session summary').fill('watch along');
  await page.getByRole('button', { name: 'Start Live' }).click();
  await expect(page.getByText('Launch Party')).toBeVisible();
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

  await page
    .getByRole('navigation', { name: 'Primary sections' })
    .getByRole('button', { name: /Profile/i })
    .click();
  await page.getByPlaceholder('Visible label').fill('Browser Author');
  await page.getByRole('button', { name: 'Save Profile' }).click();
  await expect(page.getByText('Browser Author')).toBeVisible();
});
