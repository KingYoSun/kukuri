import { expect, test } from '@playwright/test';
import { seedIndexLayout } from './community-index-layout-fixture';

for (const locale of ['ja', 'en', 'zh-CN']) {
  for (const width of [390, 1280]) {
    test(`Explore tabs fit one compact row ${locale} ${width}`, async ({ page }) => {
      await page.setViewportSize({ width, height: 844 });
      await seedIndexLayout(page, { locale });
      await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
      const tabs = page.locator('.shell-community-index-tabs');
      await expect(tabs.getByRole('tab')).toHaveCount(3);
      const geometry = await tabs.evaluate(element => ({
        rows: Array.from(element.querySelectorAll('[role="tab"]')).map(tab => {
          const rect = tab.getBoundingClientRect();
          return { top: rect.top, height: rect.height, overflow: tab.scrollWidth - tab.clientWidth };
        }),
        overflow: element.scrollWidth - element.clientWidth,
      }));
      for (const row of geometry.rows) {
        expect(row.top).toBe(geometry.rows[0].top);
        expect(row.height).toBeLessThanOrEqual(33);
        expect(row.overflow).toBeLessThanOrEqual(1);
      }
      expect(geometry.overflow).toBeLessThanOrEqual(1);
      await tabs.getByRole('tab').nth(1).click();
      await expect(tabs.getByRole('tab').nth(1)).toHaveAttribute('aria-selected', 'true');
      await tabs.getByRole('tab').nth(2).focus();
      await page.keyboard.press('Enter');
      await expect(tabs.getByRole('tab').nth(2)).toHaveAttribute('aria-selected', 'true');
    });
  }
}
