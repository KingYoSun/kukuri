import { expect, test } from '@playwright/test';

test('browser mock hash routes deep link profile, channels, and settings surfaces', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });

  await page.goto('/#/profile');
  await expect(page.getByPlaceholder('Visible label')).toBeVisible();
  await expect(page).toHaveURL(/#\/profile\?topic=/);

  await page.goto('/#/channels');
  await expect(page.getByPlaceholder('core contributors')).toBeVisible();
  await expect(page).toHaveURL(/#\/channels\?topic=/);

  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo&settings=discovery');
  const settingsDialog = page.getByRole('dialog', { name: 'Settings & diagnostics' });
  await expect(settingsDialog).toBeVisible();
  await expect(settingsDialog.getByTestId('settings-section-discovery')).toHaveAttribute(
    'aria-current',
    'location'
  );

  await settingsDialog.getByTestId('settings-section-connectivity').click();
  await expect(page).toHaveURL(/settings=connectivity/);
});

test('browser mock hash history keeps route state stable without narrow-width overflow', async ({
  page,
}) => {
  await page.setViewportSize({ width: 700, height: 980 });
  await page.goto('/');

  await page.getByTestId('shell-nav-trigger').click();
  await page
    .getByRole('navigation', { name: 'Primary sections' })
    .getByRole('button', { name: /Profile/i })
    .click();
  await expect(page.getByPlaceholder('Visible label')).toBeVisible();
  await expect(page).toHaveURL(/#\/profile\?topic=/);

  await page.goBack();
  await expect(page).toHaveURL(/#\/timeline\?topic=/);
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();

  await page.getByPlaceholder('Write a post').fill('route history post');
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page.getByText('route history post')).toBeVisible();

  await page.getByText('route history post').click();
  await expect(page).toHaveURL(/context=thread/);
  await expect(page.getByRole('tab', { name: 'Thread' })).toHaveAttribute('aria-selected', 'true');

  await page.goBack();
  await expect(page).not.toHaveURL(/context=thread/);
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();

  await page.goForward();
  await expect(page).toHaveURL(/context=thread/);

  await page.goBack();
  await page.getByTestId('shell-nav-trigger').click();
  await page
    .getByRole('navigation', { name: 'Primary sections' })
    .getByRole('button', { name: /Channels/i })
    .click();
  await expect(page).toHaveURL(/#\/channels\?topic=/);
  await expect(page.getByPlaceholder('core contributors')).toBeVisible();

  const noOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth
  );
  expect(noOverflow).toBeTruthy();
});
