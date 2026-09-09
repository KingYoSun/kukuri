import { expect, test } from '@playwright/test';

import en from '../../src/i18n/locales/en/shell.json' with { type: 'json' };
import ja from '../../src/i18n/locales/ja/shell.json' with { type: 'json' };
import zh from '../../src/i18n/locales/zh-CN/shell.json' with { type: 'json' };

import {
  expectIndexContentContained, indexLayoutCalls, LONG_NODE_URLS, seedIndexLayout,
} from './community-index-layout-fixture';

test('Column content and Control Center use body typography without shrinking headers', async ({ page }, testInfo) => {
  test.setTimeout(60_000);
  await page.setViewportSize({ width: 1280, height: 800 });
  await seedIndexLayout(page);
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  const column = page.locator('.shell-column-surface[data-active]');
  await expect(column.getByTestId('community-index-explore')).toBeVisible();
  await testInfo.attach('explore', { body: await page.screenshot(), contentType: 'image/png' });
  await expect.soft(column.locator('.shell-column-header h2')).toHaveCSS('font-size', '16px');
  await expect.soft(column.locator('.shell-column-body')).toHaveCSS('font-size', '14px');
  await expect.soft(column.getByRole('tab', { name: '検索', exact: true })).toHaveCSS('font-size', '14px');
  await expect.soft(column.getByRole('textbox')).toHaveCSS('font-size', '14px');
  await expect.soft(page.locator('.post-body').first()).toHaveCSS('font-size', '15px');
  await page.getByTestId('control-center-trigger').click();
  const center = page.locator('.shell-control-center');
  await expect(center).toBeVisible();
  await expect.soft(center.locator('.shell-control-center-grid')).toHaveCSS('font-size', '14px');
  await expect.soft(center.locator('.shell-control-center-add-grid button').filter({ hasText: '見つける' })).toHaveCSS('font-size', '14px');
  await expect.soft(center.locator('.shell-control-center-column-focus small').first()).toHaveCSS('font-size', '12px');
  await testInfo.attach('control-center', { body: await page.screenshot(), contentType: 'image/png' });
});

for (const [locale, { communityIndex: copy }] of Object.entries({ ja, en, 'zh-CN': zh })) {
  for (const theme of ['dark', 'light']) {
    for (const width of [1280, 390]) {
      test(`Explore contains long notices and all query operations in ${locale} ${theme} ${width}`, async ({ page }) => {
        await page.setViewportSize({ width, height: 800 });
        await seedIndexLayout(page, { locale, theme, pendingNodes: true, holdQueries: true });
        await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
        const workspace = page.getByTestId('community-index-explore');
        await expect(workspace.getByRole('textbox', { name: copy.queryLabel })).toBeVisible();
        const methods = ['searchCommunityNodeIndex', 'discoverCommunityNodeIndex', 'recommendCommunityNodeIndex'];
        let operation = 0;
        for (const key of ['search', 'discovery', 'recommendations'] as const) {
          const label = copy.operations[key];
          await workspace.getByRole('tab', { name: label, exact: true }).click();
          if (operation === 0) await workspace.getByRole('textbox').fill('layout-no-matching-post');
          await expectIndexContentContained(workspace);
          const submit = workspace.getByRole('button', { name: copy.run, exact: true });
          await submit.scrollIntoViewIfNeeded();
          if (operation === 0) {
            await submit.click();
          } else {
            await submit.focus();
            await page.keyboard.press(operation === 1 ? 'Enter' : 'Space');
          }
          await expect(workspace.locator('button[type="submit"]')).toBeDisabled();
          await expectIndexContentContained(workspace);
          expect(await indexLayoutCalls(page)).toEqual(methods.slice(0, operation + 1).map(method => ({
            method, baseUrl: 'https://api.kukuri.app',
          })));
          await page.evaluate(() => (window as unknown as { __releaseIndexLayoutQuery: () => void }).__releaseIndexLayoutQuery());
          await expect(submit).toBeEnabled();
          if (operation === 0) await expect(workspace.getByText(copy.empty, { exact: true })).toBeVisible();
          operation += 1;
        }
        await workspace.getByRole('tab', { name: copy.operations.search, exact: true }).click();
        await expect(workspace.getByRole('textbox')).toHaveValue('layout-no-matching-post');
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      });
    }
  }
}

test('Explore preserves the query and Column arrangement through resizing and reflow', async ({ page }) => {
  await seedIndexLayout(page);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  const workspace = page.getByTestId('community-index-explore');
  const query = workspace.getByRole('textbox');
  await query.fill('入力中の検索語');
  const columns = await page.evaluate(() => JSON.parse(localStorage.getItem('kukuri:workspace-layout:v1')!).columns);
  for (const width of [900, 760, 390, 1280]) {
    await page.setViewportSize({ width, height: 800 });
    await expect(query).toHaveValue('入力中の検索語');
    await expectIndexContentContained(workspace);
  }
  await page.evaluate(() => { document.documentElement.style.zoom = '2'; });
  await expectIndexContentContained(workspace);
  await query.focus();
  await page.keyboard.press('Tab');
  await expect(workspace.locator('button[type="submit"]')).toBeFocused();
  await page.evaluate(() => { document.documentElement.style.zoom = ''; });
  await expect(query).toHaveValue('入力中の検索語');
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('kukuri:workspace-layout:v1')!).columns)).toEqual(columns);
  expect(await indexLayoutCalls(page)).toEqual([]);
});

test('Control Center returns to the query context and retains the saved layout', async ({ page }) => {
  await seedIndexLayout(page);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  const workspace = page.getByTestId('community-index-explore');
  const query = workspace.getByRole('textbox');
  await query.fill('入力中の検索語');
  const columns = await page.evaluate(() => JSON.parse(localStorage.getItem('kukuri:workspace-layout:v1')!).columns);
  const activeColumn = await page.evaluate(() => JSON.parse(localStorage.getItem('kukuri:workspace-layout:v1')!).activeColumnId);
  const trigger = page.getByTestId('control-center-trigger');
  await trigger.click();
  const center = page.locator('.shell-control-center');
  await center.getByRole('button', { name: '設定', exact: true }).click();
  const settings = page.getByRole('dialog', { name: '設定', exact: true });
  await expect(settings).toBeVisible();
  await expect(settings.locator('select').first()).toHaveCSS('font-size', '16px');
  await page.keyboard.press('Escape');
  await expect(settings).not.toBeVisible();
  if (await center.isVisible()) await page.keyboard.press('Escape');
  await expect(trigger).toBeFocused();
  await expect(query).toHaveValue('入力中の検索語');
  expect(await indexLayoutCalls(page)).toEqual([]);
  const restored = await page.evaluate(() => JSON.parse(localStorage.getItem('kukuri:workspace-layout:v1')!));
  expect(restored.columns).toEqual(columns);
  expect(restored.activeColumnId).toBe(activeColumn);
  await page.reload();
  await expect(workspace).toBeVisible();
  await expect(page.locator('.shell-column-surface')).toHaveCount(3);
});

test('the focused query action clears the fixed mobile controls at the 200 percent reflow size', async ({ page }) => {
  await seedIndexLayout(page);
  await page.setViewportSize({ width: 640, height: 400 });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  const workspace = page.getByTestId('community-index-explore');
  await workspace.getByRole('textbox').focus();
  await page.keyboard.press('Tab');
  const submit = workspace.locator('button[type="submit"]');
  await expect(submit).toBeFocused();
  const visibility = await submit.evaluate(button => {
    const box = button.getBoundingClientRect();
    const pane = button.closest('.shell-column-body')!.getBoundingClientRect();
    const occluded = [0.15, 0.5, 0.85].some(fraction => {
      const hit = document.elementFromPoint(box.left + box.width * fraction, box.top + box.height / 2);
      return !hit || !button.contains(hit);
    });
    return { clipped: box.top < pane.top || box.bottom > pane.bottom, occluded };
  });
  expect(visibility).toEqual({ clipped: false, occluded: false });
  const pane = await page.locator('.shell-column-body').filter({ has: workspace }).boundingBox();
  const indicator = await page.locator('.shell-column-page-indicator').boundingBox();
  expect(pane).not.toBeNull();
  expect(indicator).not.toBeNull();
  expect(pane!.y + pane!.height).toBeLessThanOrEqual(indicator!.y);
});

for (const width of [1280, 760, 390]) {
  test(`long per-node policy labels remain readable and target the selected node at ${width}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 800 });
    await seedIndexLayout(page, { pendingNodes: true });
    await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
    const workspace = page.getByTestId('community-index-explore');
    const policies = workspace.getByRole('button', { name: `規約を確認: ${LONG_NODE_URLS[1]}`, exact: true });
    await expect(policies).toBeVisible();
    await testInfo.attach('long-policy-label', { body: await page.screenshot(), contentType: 'image/png' });
    await expectIndexContentContained(workspace);
    expect(await indexLayoutCalls(page)).toEqual([]);
    await policies.click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    expect(await indexLayoutCalls(page)).toEqual([{ method: 'policies', baseUrl: LONG_NODE_URLS[1] }]);
    await page.keyboard.press('Escape');
    await expect(dialog).not.toBeVisible();
    expect(await indexLayoutCalls(page)).toEqual([{ method: 'policies', baseUrl: LONG_NODE_URLS[1] }]);
  });
}
