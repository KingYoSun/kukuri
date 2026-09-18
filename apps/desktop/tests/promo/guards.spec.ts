import { expect, test } from '@playwright/test';

import {
  assertNoBrokenGlyphs,
  captureStableScreen,
  waitForRenderReady,
} from './fixtures/captureScene';

/**
 * 撮影の guard が、不完全な画面を素材として採用しないことを確かめる (#1039 AC-4 / TR-3)。
 *
 * アプリは使わず、問題のある画面を setContent で直接作る。撮影の出力 (captures) には書かない。
 */

test('読み込めない画像がある画面は採用しない', async ({ page }) => {
  await page.setContent('<p>demo</p><img src="data:image/png;base64,AAAA" alt="">');
  await expect(waitForRenderReady(page)).rejects.toThrow('読み込みが終わっていない画像');
});

test('文字の無い画面は採用しない', async ({ page }) => {
  await page.setContent('<div style="width: 120px; height: 120px; background: #121212"></div>');
  await expect(waitForRenderReady(page)).rejects.toThrow('画面に文字が無い');
});

test('置換文字が出ている画面は採用しない', async ({ page }) => {
  await page.setContent(`<p>demo ${String.fromCharCode(0xfffd)} text</p>`);
  await expect(assertNoBrokenGlyphs(page)).rejects.toThrow('置換文字');
});

test('描き変わり続ける画面は採用しない', async ({ page }) => {
  await page.setContent(
    '<p id="n">0</p>' +
      '<script>let i = 0; setInterval(() => { document.getElementById("n").textContent = String(++i); }, 40);</script>'
  );
  await expect(captureStableScreen(page)).rejects.toThrow('画面が落ち着かない');
});

test('描き終わって落ち着いた画面は採用する', async ({ page }) => {
  await page.setContent('<p>demo</p>');
  await waitForRenderReady(page);
  await assertNoBrokenGlyphs(page);
  const png = await captureStableScreen(page);
  expect(png.length).toBeGreaterThan(0);
});
