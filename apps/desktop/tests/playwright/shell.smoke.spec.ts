import { expect, test } from '@playwright/test';

test('browser mock shell can publish, open thread, and update discovery from settings', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await expect(page.getByRole('heading', { name: /Seeded DHT/i })).toBeVisible();

  await page.getByPlaceholder('Write a post').fill('hello browser mock');
  await page.getByRole('button', { name: 'Publish' }).click();

  await expect(page.getByText('hello browser mock')).toBeVisible();

  await page.getByText('hello browser mock').click();
  await expect(page.getByRole('tab', { name: 'Thread' })).toHaveAttribute('aria-selected', 'true');

  await page.getByTestId('shell-settings-trigger').click();
  await expect(page.getByRole('dialog', { name: 'Settings & diagnostics' })).toBeVisible();
  await page.getByTestId('settings-section-discovery').click();
  await page.getByPlaceholder('node_id or node_id@host:port').fill('seed-peer-1');
  await page.getByRole('button', { name: 'Save Seeds' }).click();

  await expect(page.getByRole('textbox', { name: 'Seed Peers' })).toHaveValue('seed-peer-1');
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
  await expect(page.getByText('Active topic: kukuri:topic:demo')).toBeVisible();

  await page.getByPlaceholder('Write a post').fill('narrow browser mock');
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page.getByText('narrow browser mock')).toBeVisible();

  await page.getByText('narrow browser mock').click();
  await expect(page.getByRole('tabpanel', { name: 'Thread' })).toBeVisible();

  await page
    .getByRole('tabpanel', { name: 'Thread' })
    .getByRole('button', { name: 'ffffffffffff' })
    .first()
    .click();
  await expect(page.getByRole('tab', { name: 'Author' })).toHaveAttribute('aria-selected', 'true');

  await page.keyboard.press('Escape');
  await page.getByTestId('shell-settings-trigger').click();
  await page.getByTestId('settings-section-connectivity').click();
  await page.getByPlaceholder('nodeid@127.0.0.1:7777').fill('peer-b@127.0.0.1:8888');
  await page.getByRole('button', { name: 'Import Peer' }).click();
  await expect(page.getByPlaceholder('nodeid@127.0.0.1:7777')).toHaveValue('');

  const noOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth
  );
  expect(noOverflow).toBeTruthy();
});
