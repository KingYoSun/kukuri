import { expect, test } from '@playwright/test';

import en from '../../src/i18n/locales/en/settings.json' with { type: 'json' };
import ja from '../../src/i18n/locales/ja/settings.json' with { type: 'json' };
import zh from '../../src/i18n/locales/zh-CN/settings.json' with { type: 'json' };

const sections = ['connectivity', 'discovery', 'community-node'] as const;

test('narrow settings keep the first diagnostic action above the workspace dock', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('kukuri.desktop.locale', 'en');
    localStorage.setItem('kukuri.desktop.theme', 'light');
    localStorage.setItem('kukuri.desktop.developer-mode', 'false');
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=developer');
  const drawer = page.getByRole('dialog');
  await drawer.getByRole('checkbox').check();
  const action = drawer.getByRole('button', { name: 'Connection diagnostics' });
  // Do not scroll first: Playwright auto-scrolling can hide a dock overlap.
  await expect(action).toBeInViewport();
  expect(await action.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    return element.contains(document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2));
  })).toBe(true);
});

for (const [locale, copy] of [['en', en], ['ja', ja], ['zh-CN', zh]] as const) {
  for (const theme of ['dark', 'light']) {
    for (const width of [390, 1280]) {
      test(`developer state and diagnostics ${locale} ${theme} ${width}`, async ({ page }) => {
        await page.addInitScript(({ locale, theme }) => {
          localStorage.setItem('kukuri.desktop.locale', locale);
          localStorage.setItem('kukuri.desktop.theme', theme);
          // Preserve mode changes across the reload below.
          if (localStorage.getItem('kukuri.desktop.developer-mode') === null) {
            localStorage.setItem('kukuri.desktop.developer-mode', 'false');
          }
        }, { locale, theme });
        await page.setViewportSize({ width, height: width === 390 ? 844 : 800 });
        await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=developer');
        const drawer = page.getByRole('dialog');
        const toggle = drawer.getByRole('checkbox', { name: copy.developer.mode.label });
        await expect(drawer.getByRole('status')).toHaveText(copy.developer.mode.disabled);
        await expect(drawer.getByRole('button', { name: copy.developer.diagnostics.connectivity })).toHaveCount(0);
        await toggle.check();
        await expect(drawer.getByRole('status')).toHaveText(copy.developer.mode.enabled);
        await expect(drawer.getByTestId('settings-section-developer')).toHaveAttribute('aria-current', 'location');
        await expect(page.locator('html')).toHaveAttribute('lang', locale);
        await expect(page.locator('html')).toHaveAttribute('data-theme', theme);

        for (const section of sections) {
          const button = drawer.getByRole('button', { name: copy.developer.diagnostics[section] });
          await button.scrollIntoViewIfNeeded();
          await expect(button).toBeInViewport();
          expect(await button.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
          await button.click();
          await expect(drawer).toBeVisible();
          await expect(drawer.getByTestId(`settings-section-${section}`)).toBeFocused();
          await expect(page).toHaveURL(new RegExp(`settings=${section}`));
          await drawer.getByTestId('settings-section-developer').click();
          await expect(drawer.getByRole('status')).toHaveText(copy.developer.mode.enabled);
        }
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        await page.reload();
        await expect(drawer.getByRole('status')).toHaveText(copy.developer.mode.enabled);
        await toggle.uncheck();
        await expect(drawer.getByRole('status')).toHaveText(copy.developer.mode.disabled);
        await expect(drawer.getByRole('button', { name: copy.developer.diagnostics.connectivity })).toHaveCount(0);
        await page.reload();
        await expect(drawer.getByRole('status')).toHaveText(copy.developer.mode.disabled);
      });
    }
  }
}

test('keyboard enables mode, visits each diagnosis and restores focus on close', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('kukuri.desktop.locale', 'en');
    localStorage.setItem('kukuri.desktop.developer-mode', 'false');
  });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  await page.getByTestId('control-center-trigger').click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const drawer = page.getByRole('dialog', { name: 'Settings' });
  const developer = drawer.getByTestId('settings-section-developer');
  await developer.click();
  // #967: Developer is the last nav item, so one Tab reaches the mode checkbox.
  await page.keyboard.press('Tab'); // Mode checkbox
  await expect(drawer.getByRole('checkbox')).toBeFocused();
  await page.keyboard.press('Space');
  await expect(drawer.getByRole('status')).toHaveText('Developer mode is on.');
  await expect(drawer.getByRole('checkbox')).toBeFocused();
  for (const [index, section] of sections.entries()) {
    for (let step = 0; step <= index; step++) await page.keyboard.press('Tab');
    await expect(drawer.getByRole('button', { name: en.developer.diagnostics[section] })).toBeFocused();
    await page.keyboard.press(index === 1 ? 'Space' : 'Enter');
    await expect(drawer.getByTestId(`settings-section-${section}`)).toBeFocused();
    // Follow the existing nav's natural tab order back to Developer.
    for (let step = 0; step < 5 - index; step++) await page.keyboard.press('Tab');
    await expect(developer).toBeFocused();
    await page.keyboard.press('Enter');
    await page.keyboard.press('Tab');
    await expect(drawer.getByRole('checkbox')).toBeFocused();
  }
  await page.keyboard.press('Escape');
  await expect(drawer).not.toBeVisible();
  await expect(page.getByTestId('control-center-trigger')).toBeFocused();
});
