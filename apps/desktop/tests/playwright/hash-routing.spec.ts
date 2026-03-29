import { expect, test, type Page } from '@playwright/test';

async function openChannelSection(page: Page) {
  const trigger = page.locator('.shell-nav-accordion-trigger').first();
  if ((await trigger.getAttribute('aria-expanded')) !== 'true') {
    await trigger.click();
  }
}

test('browser mock hash routes deep link profile, timeline normalization, and settings surfaces', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });

  await page.goto('/#/profile');
  await expect(page.getByRole('button', { name: 'Edit Profile' })).toBeVisible();
  await expect(page).toHaveURL(/#\/profile\?topic=/);

  await page.goto('/#/channels');
  await expect(page.locator('.shell-nav-accordion-trigger').first()).toBeVisible();
  await expect(page).toHaveURL(/#\/timeline\?topic=/);

  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo&settings=appearance');
  const settingsDialog = page.getByRole('dialog', { name: 'Settings & diagnostics' });
  await expect(settingsDialog).toBeVisible();
  await expect(settingsDialog.getByTestId('settings-section-appearance')).toHaveAttribute(
    'aria-current',
    'location'
  );
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

  await settingsDialog.getByTestId('settings-section-connectivity').click();
  await expect(page).toHaveURL(/settings=connectivity/);
});

test('browser mock hash history keeps route state stable without narrow-width overflow', async ({
  page,
}) => {
  await page.setViewportSize({ width: 700, height: 980 });
  await page.goto('/');

  await expect(page.getByTestId('shell-nav-trigger')).toBeVisible();
  await page.getByRole('tab', { name: 'Profile' }).click();
  await expect(page.getByRole('button', { name: 'Edit Profile' })).toBeVisible();
  await expect(page).toHaveURL(/#\/profile\?topic=/);

  await page.goBack();
  await expect(page).toHaveURL(/#\/timeline\?topic=/);
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();

  await page.getByPlaceholder('Write a post').fill('route history post');
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page.getByText('route history post')).toBeVisible();

  await page.getByText('route history post').click();
  await expect(page).toHaveURL(/context=thread/);
  await expect(page.getByRole('complementary', { name: 'Thread' })).toBeVisible();

  await page.goBack();
  await expect(page).not.toHaveURL(/context=thread/);
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();

  await page.goForward();
  await expect(page).toHaveURL(/context=thread/);
  await expect(page.getByRole('complementary', { name: 'Thread' })).toBeVisible();

  await page.goBack();
  await page.getByTestId('shell-nav-trigger').click();
  await openChannelSection(page);
  await page.getByPlaceholder('core contributors').fill('Route Room');
  await page.getByRole('button', { name: 'Create Channel' }).click();
  await expect(page).toHaveURL(/#\/timeline\?topic=.*&channel=channel-/);

  const noOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth <= window.innerWidth
  );
  expect(noOverflow).toBeTruthy();
});
