import { expect, test } from '@playwright/test';

const image = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a/A8AAAAASUVORK5CYII=', 'base64');

test('settings language localizes Explore and the file control across reloads', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/timeline?settings=appearance');
  await page.getByRole('dialog', { name: 'Settings', exact: true }).getByLabel('Language').selectOption('ja');
  await page.keyboard.press('Escape');
  await page.goto('/#/explore');
  await expect(page.getByRole('heading', { name: 'コミュニティインデックス', exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'コミュニティインデックス', exact: true })).toBeVisible();
  await page.goto('/#/timeline');
  await page.locator('[data-column-id][aria-current="true"] .shell-column-primary-action').click();
  await expect(page.getByRole('button', { name: 'ファイルを選択', exact: true })).toBeVisible();
  await expect(page.getByText('ファイル未選択', { exact: true })).toBeVisible();
});

for (const locale of [
  { code: 'ja', choose: 'ファイルを選択', empty: 'ファイル未選択', selected: '添付 1 件', remove: '削除' },
  { code: 'en', choose: 'Choose files', empty: 'No files selected', selected: 'Attached files: 1', remove: 'Remove' },
  { code: 'zh-CN', choose: '选择文件', empty: '未选择文件', selected: '已附加 1 个文件', remove: '移除' },
]) {
  for (const theme of ['dark', 'light']) {
    test(`${locale.code} ${theme} file picker supports pointer, keyboard, reset and narrow layouts`, async ({ page }, testInfo) => {
      await page.addInitScript(({ code, theme }) => {
        localStorage.setItem('kukuri.desktop.locale', code);
        localStorage.setItem('kukuri.desktop.theme', theme);
      }, { code: locale.code, theme });
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto('/#/timeline');
      await page.locator('[data-column-id][aria-current="true"] .shell-column-primary-action').click();
      const composer = page.locator('.composer');
      const button = composer.getByRole('button', { name: locale.choose, exact: true });
      await expect(composer.getByText(locale.empty, { exact: true })).toBeVisible();
      await expect(composer.locator('input[type=file]')).toBeHidden();
      await composer.locator('textarea').fill('preserved draft');
      await testInfo.attach('empty', { body: await composer.screenshot(), contentType: 'image/png' });
      const file = { name: '長いファイル名'.repeat(12) + '.png', mimeType: 'image/png', buffer: image };
      for (const action of ['pointer', 'Enter', 'Space']) {
        const chooserPromise = page.waitForEvent('filechooser');
        if (action === 'pointer') await button.click();
        else { await button.focus(); await button.press(action); }
        const chooser = await chooserPromise;
        expect(chooser.isMultiple()).toBe(true);
        await chooser.setFiles(file);
        await expect(composer.getByText(locale.selected, { exact: true })).toBeVisible();
        await expect(composer.locator('input[type=file]')).toHaveValue('');
        await expect(composer.locator('textarea')).toHaveValue('preserved draft');
        await expect(button).toBeFocused();
        for (const width of [1280, 900, 390]) {
          await page.setViewportSize({ width, height: 800 });
          await expect(button).toBeVisible();
          expect(await composer.evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
        }
        await testInfo.attach(`selected-${action}`, { body: await composer.screenshot(), contentType: 'image/png' });
        await composer.getByRole('button', { name: locale.remove, exact: true }).click();
        await expect(composer.getByText(locale.empty, { exact: true })).toBeVisible();
      }
    });
  }
}
