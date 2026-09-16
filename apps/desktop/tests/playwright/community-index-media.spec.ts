import { expect, test, type Page } from '@playwright/test';

import {
  OBJECT_ID,
  SHOT_DIR,
  SHOT_PREFIX,
  runExploreSearch,
  seedExploreMedia,
} from './community-index-media-fixture';

// #1052: 「見つける」の解決済み投稿が、タイムラインと同じ表示経路で添付画像を描画し、
// 成人向けラベル付きでは表示設定 OFF の間プレースホルダーのままであることを実ブラウザで確認する。
// 視覚回帰 baseline は増やさず、記録用の画像だけを環境変数指定時に書き出す。

async function capture(page: Page, name: string) {
  if (!process.env.KUKURI_1052_SHOTS) return;
  await page.screenshot({ path: `${SHOT_DIR}/${SHOT_PREFIX}-${name}.png` });
}

test('resolved Explore results render their attachment and open the image viewer', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'dark' });
  const explore = await runExploreSearch(page);

  const preview = explore.getByTestId(`media-preview-${OBJECT_ID}`);
  await expect(preview).toBeVisible();
  await expect(preview).toHaveJSProperty('naturalWidth', 96);
  await capture(page, 'ja-dark-1400');

  // 画像はキーボードからも開ける(共有 PostCard の viewer をそのまま使う)。
  const trigger = explore.locator('.media-image-trigger').first();
  await trigger.focus();
  await expect(trigger).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog')).toBeVisible();
  await capture(page, 'ja-dark-1400-viewer');
  await page.keyboard.press('Escape');
});

test('resolved Explore results keep the attachment inside a narrow column', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'light' });
  const explore = await runExploreSearch(page);

  const preview = explore.getByTestId(`media-preview-${OBJECT_ID}`);
  await expect(preview).toBeVisible();
  const overflow = await explore.evaluate((root) => root.scrollWidth - root.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  await capture(page, 'ja-light-390');
});

test('adult-labeled Explore results stay gated while the display setting is off', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'dark', adultLabeled: true });
  const explore = await runExploreSearch(page);

  await expect(explore.getByTestId(`media-adult-gated-${OBJECT_ID}`)).toBeVisible();
  await expect(explore.getByTestId(`media-preview-${OBJECT_ID}`)).toHaveCount(0);
  await capture(page, 'ja-dark-1400-adult-gated');
});
