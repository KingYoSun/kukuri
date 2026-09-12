import { expect, test, type Page } from '@playwright/test';

const THEME_KEY = 'kukuri.desktop.theme';

async function openAppearance(page: Page) {
  await page.getByTestId('control-center-trigger').click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Settings', exact: true });
  await dialog.getByTestId('settings-section-appearance').click();
  return dialog;
}

for (const width of [1600, 390]) {
  test(`theme switch preserves Column context and applies to portals at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/');
    const timeline = page.locator('.shell-column-surface[data-column-id^="column:timeline:"]').first();
    await expect(timeline).toBeVisible();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    const snapshot = () => page.locator('.shell-column-surface').evaluateAll((columns) => columns.map((column) => ({
      id: column.getAttribute('data-column-id'), pinned: column.hasAttribute('data-pinned'),
      span: column.getAttribute('data-span'), width: column.getBoundingClientRect().width,
      text: column.querySelector('.shell-column-header')?.textContent,
    })));
    const before = await snapshot();

    await timeline.locator('.shell-column-primary-action').click();
    await page.getByPlaceholder('Write a post').fill('A draft that survives a theme change');
    await page.keyboard.press('Escape');

    for (const theme of ['light', 'dark'] as const) {
      const settings = await openAppearance(page);
      await settings.getByRole('radio', { name: theme === 'light' ? /Light/i : /Dark/i }).click();
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      await expect.poll(() => page.evaluate((key) => localStorage.getItem(key), THEME_KEY)).toBe(theme);
      const panel = theme === 'dark' ? 'rgb(33, 33, 33)' : 'rgb(255, 255, 255)';
      await expect(timeline).toHaveCSS('background-color', panel);
      await expect(settings).toHaveCSS('background-color', panel);
      expect(await settings.evaluate((e) => e.closest('.shell-phase1'))).toBeNull();
      await page.keyboard.press('Escape');
      await expect(settings).not.toBeVisible();
      expect(await snapshot()).toEqual(before);

      await timeline.locator('.shell-column-primary-action').click();
      const draft = page.getByPlaceholder('Write a post');
      await expect(draft).toHaveValue('A draft that survives a theme change');
      await expect(draft).toHaveCSS('border-top-color', theme === 'dark' ? 'rgb(131, 131, 126)' : 'rgb(133, 133, 127)');
      await draft.focus();
      await expect(draft).toBeFocused();
      await page.keyboard.press('Escape');
      await expect(timeline.locator('.shell-column-primary-action')).toBeFocused();
      await testInfo.attach(`column-${theme}`, { body: await timeline.screenshot(), contentType: 'image/png' });
    }
    // Reload may restore the draft as an inline composer; do not replace that existing behavior.
    // No init script re-injects the theme. The existing shell smoke also checks light restoration.
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(timeline).toHaveCSS('background-color', 'rgb(33, 33, 33)');
  });
}

test('invalid saved theme retains the existing dark fallback', async ({ page }) => {
  await page.addInitScript((key) => localStorage.setItem(key, 'invalid'), THEME_KEY);
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('.shell-column-surface').first()).toHaveCSS('background-color', 'rgb(33, 33, 33)');
});
