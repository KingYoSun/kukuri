import { expect, test } from '@playwright/test';

test('browser mock shell can publish and render a post', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: /Seeded DHT/i })).toBeVisible();

  await page.getByPlaceholder('Write a post').fill('hello browser mock');
  await page.getByRole('button', { name: 'Publish' }).click();

  await expect(page.getByText('hello browser mock')).toBeVisible();
  await expect(page.getByText('Configured Peers', { exact: true })).toBeVisible();
});
