import { expect, test } from '@playwright/test';

for (const input of ['pointer', 'keyboard'] as const) {
  test(`browser resource links preserve native anchor behavior with ${input}`, async ({ page, context }) => {
    // No external request is made by this contract test.
    await context.route('https://github.com/**', (route) => route.fulfill({
      contentType: 'text/html', body: '<title>Resource fixture</title><p>Resource fixture</p>',
    }));
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=release');
    const dialog = page.getByRole('dialog', { name: 'Settings' });
    const link = dialog.getByRole('link', { name: 'Latest release' });
    await expect(link).toBeVisible();
    const originalUrl = page.url();
    const popupPromise = page.waitForEvent('popup');
    if (input === 'keyboard') {
      await link.focus();
      await page.keyboard.press('Enter');
    } else {
      await link.click();
    }
    const popup = await popupPromise;
    await expect(popup).toHaveURL('https://github.com/KingYoSun/kukuri/releases/latest');
    await expect(popup).toHaveTitle('Resource fixture');
    await expect(page).toHaveURL(originalUrl);
    await expect(dialog).toBeVisible();
    await popup.close();
  });
}
