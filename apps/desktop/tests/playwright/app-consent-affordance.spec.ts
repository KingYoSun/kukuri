import { expect, test } from '@playwright/test';
import { appConsentCalls, seedAppConsent } from './app-consent-fixture';

const labels = {
  en: { blocked: 'Age confirmation required', ready: 'Accept and continue' },
  ja: { blocked: '年齢確認が必要', ready: '同意して続行' },
  'zh-CN': { blocked: '需要确认年龄', ready: '同意并继续' },
};

for (const locale of ['en', 'ja', 'zh-CN'] as const) {
  for (const theme of ['dark', 'light']) {
    test(`consent blocked action is unmistakable before input and after unchecking: ${locale} ${theme}`, async ({ page }, testInfo) => {
      await seedAppConsent(page, { locale, theme });
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto('/');
      const checkbox = page.getByRole('checkbox');
      // The same physical button survives label changes, including pending.
      const accept = page.locator('.app-consent-actions').getByRole('button').first();
      await expect(checkbox).toBeVisible();
      const palette = await page.evaluate(() => {
        const probe = document.createElement('span');
        probe.style.display = 'none';
        document.body.append(probe);
        const color = (token: string) => {
          probe.style.color = `var(${token})`;
          return getComputedStyle(probe).color;
        };
        const result = {
          blocked: color('--surface-panel-muted'),
          primary: color('--surface-button-primary'),
          boundary: color('--muted-foreground-soft'),
        };
        probe.remove();
        return result;
      });

      for (const state of ['initial', 'after-unchecking']) {
        await expect(accept).toBeDisabled();
        await expect(accept).toHaveAccessibleName(labels[locale].blocked);
        await expect(accept).toHaveCSS('background-color', palette.blocked);
        await expect(accept).toHaveCSS('background-image', 'none');
        await expect(accept).toHaveCSS('box-shadow', 'none');
        await expect(accept).toHaveCSS('border-top-style', 'dashed');
        await expect(accept).toHaveCSS('border-top-color', palette.boundary);
        const box = await accept.boundingBox();
        expect(box).not.toBeNull();
        // A disabled button is not Playwright-actionable; use actual pointer input.
        await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
        await page.mouse.down();
        await page.mouse.up();
        await expect(accept).toHaveAccessibleName(labels[locale].blocked);
        await expect(accept).toHaveCSS('background-color', palette.blocked);
        expect(await appConsentCalls(page)).toEqual([]);
        await testInfo.attach(`${state}-blocked`, {
          body: await accept.screenshot(), contentType: 'image/png',
        });

        await checkbox.focus();
        await page.keyboard.press('Space');
        await expect(accept).toBeEnabled();
        await expect(accept).toHaveAccessibleName(labels[locale].ready);
        await expect(accept).toHaveCSS('background-color', palette.primary);
        await expect(accept).not.toHaveCSS('box-shadow', 'none');
        await expect(accept).not.toHaveCSS('border-top-style', 'dashed');
        expect(await appConsentCalls(page)).toEqual([]);
        await testInfo.attach(`${state}-ready`, {
          body: await accept.screenshot(), contentType: 'image/png',
        });
        await page.keyboard.press('Space');
      }
      await page.reload();
      await expect(checkbox).not.toBeChecked();
      await expect(accept).toHaveAccessibleName(labels[locale].blocked);
      await expect(accept).toBeDisabled();
      expect(await appConsentCalls(page)).toEqual([]);
    });
  }
}
