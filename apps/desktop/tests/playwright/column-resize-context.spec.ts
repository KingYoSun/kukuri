import { expect, test, type Locator, type Page } from '@playwright/test';

import en from '../../src/i18n/locales/en/shell.json' with { type: 'json' };
import ja from '../../src/i18n/locales/ja/shell.json' with { type: 'json' };
import zh from '../../src/i18n/locales/zh-CN/shell.json' with { type: 'json' };

import { expectIndexContentContained, indexLayoutCalls, seedIndexLayout } from './community-index-layout-fixture';

const layoutKey = 'kukuri:workspace-layout:v1';

// Seed once through the page, not an init script which would rewrite the saved
// layout on reload. Begin on Timeline so navigation must reveal Explore itself.
async function startThreeColumns(page: Page, locale = 'ja', theme = 'dark', explore = ja.primarySections.explore) {
  await seedIndexLayout(page, { threeColumns: false, locale, theme });
  await page.goto('/');
  await expect(page.locator('.shell-column-surface')).toHaveCount(5);
  await page.evaluate(key => {
    const layout = JSON.parse(localStorage.getItem(key)!);
    layout.columns = ['timeline', 'notifications', 'explore'].map(kind =>
      layout.columns.find((column: { kind: string }) => column.kind === kind));
    layout.activeColumnId = layout.columns[0].id;
    localStorage.setItem(key, JSON.stringify(layout));
  }, layoutKey);
  await page.goto('/');
  await expect(page.locator('.shell-column-surface')).toHaveCount(3);
  await page.getByTestId('control-center-trigger').click();
  await page.locator('.shell-control-center-column-focus').filter({ hasText: explore }).click();
  const column = page.locator('.shell-column-surface').filter({ has: page.getByTestId('community-index-explore') });
  await expect(column).toHaveAttribute('data-active', 'true');
  await expect(page.locator('.shell-control-center')).not.toBeVisible();
  return column;
}

async function expectColumnInCanvas(column: Locator) {
  // Reading geometry must not focus or scroll the target into view first.
  await expect.poll(() => column.evaluate(element => {
    const box = element.getBoundingClientRect();
    const canvas = element.closest('.shell-column-canvas')!.getBoundingClientRect();
    return box.left >= Math.max(0, canvas.left) - 1 && box.right <= Math.min(innerWidth, canvas.right) + 1;
  })).toBe(true);
}

for (const [locale, copy] of Object.entries({ ja, en, 'zh-CN': zh })) {
  for (const theme of ['dark', 'light']) {
    test(`resizing from a fitting desktop layout keeps Explore controls in the Canvas before interaction (${locale}, ${theme})`, async ({ page }) => {
      await page.setViewportSize({ width: 1600, height: 800 });
      const column = await startThreeColumns(page, locale, theme, copy.primarySections.explore);
      const workspace = column.getByTestId('community-index-explore');
      const query = workspace.getByRole('textbox');
      await query.fill('入力を保持');
      await expectColumnInCanvas(column);
      for (const width of [1280, 900, 760, 1280]) {
        await page.setViewportSize({ width, height: 800 });
        await expectColumnInCanvas(column);
        await expect(query).toBeFocused();
        await expect(query).toHaveValue('入力を保持');
        await expectIndexContentContained(workspace);
        expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
  }
  expect(await indexLayoutCalls(page)).toEqual([]);
});

test(`crossing the mobile breakpoint preserves the selected Column and its saved layout without reseeding (${locale}, ${theme})`, async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  const column = await startThreeColumns(page, locale, theme, copy.primarySections.explore);
  const query = column.getByRole('textbox');
  await query.fill('境界を往復');
  const saved = await page.evaluate(key => JSON.parse(localStorage.getItem(key)!), layoutKey);
  for (const width of [759, 390, 760, 1280]) {
    await page.setViewportSize({ width, height: 800 });
    await expectColumnInCanvas(column);
    // Let the existing scroll-settle timer run: an early assertion would miss
    // the later activation of a different Column caused by browser scroll snap.
    await page.waitForTimeout(250);
    await expect(column).toHaveAttribute('data-active', 'true');
    await expect(query).toBeFocused();
    await expect(query).toHaveValue('境界を往復');
  }
  expect(await indexLayoutCalls(page)).toEqual([]);
  expect(await page.evaluate(key => JSON.parse(localStorage.getItem(key)!), layoutKey)).toEqual(saved);
  await page.reload();
  await expect(page.locator('.shell-column-surface')).toHaveCount(3);
  await expect(column).toHaveAttribute('data-active', 'true');
  await expectColumnInCanvas(column);
  expect(await page.evaluate(key => JSON.parse(localStorage.getItem(key)!), layoutKey)).toEqual(saved);
});
  }
}

test('ordinary Canvas scrolling is not pulled back to an offscreen active Column on resize', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 800 });
  const column = await startThreeColumns(page);
  await expectColumnInCanvas(column);
  const canvas = page.locator('.shell-column-canvas');
  await canvas.hover();
  await page.mouse.wheel(-1600, 0);
  await expect.poll(() => canvas.evaluate(element => element.scrollLeft)).toBe(0);
  await page.waitForTimeout(250);
  await expect(column).toHaveAttribute('data-active', 'true');
  await page.setViewportSize({ width: 1000, height: 800 });
  await page.waitForTimeout(250);
  expect(await canvas.evaluate(element => element.scrollLeft)).toBe(0);
});
