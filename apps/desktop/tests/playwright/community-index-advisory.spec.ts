import { expect, test, type Page } from '@playwright/test';

import {
  OBJECT_ID,
  runExploreSearch,
  seedExploreMedia,
} from './community-index-media-fixture';

// #1055: 「見つける」で content advisory 付きの結果が、self-label と同じ代替表示になり、
// 発行 node / 分類 / 確信度 / 根拠と異議申し立て導線を説明することを実ブラウザで確認する。
// 視覚回帰 baseline は増やさず、記録用の画像だけを環境変数指定時に書き出す。

const SHOT_DIR = '../../docs/ui-reviews/assets/1055';
const SHOT_PREFIX = process.env.KUKURI_1055_SHOT_PREFIX ?? 'after';

async function capture(page: Page, name: string) {
  if (!process.env.KUKURI_1055_SHOTS) return;
  await page.screenshot({ path: `${SHOT_DIR}/${SHOT_PREFIX}-${name}.png` });
}

test('advisory-labeled Explore results are gated and explain the issuing node', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'dark', advisoryLabeled: true });
  const explore = await runExploreSearch(page);

  const placeholder = explore.getByTestId(`media-adult-gated-${OBJECT_ID}`);
  await expect(placeholder).toBeVisible();
  await expect(explore.getByTestId(`media-preview-${OBJECT_ID}`)).toHaveCount(0);

  // #1108: 説明は枠から開く詳細 dialog に置く。
  await placeholder.click();
  const dialog = page.getByRole('dialog', { name: 'コミュニティノードによる推定' });
  const advisory = dialog.getByTestId(`post-advisory-gated-${OBJECT_ID}`);
  await expect(advisory).toBeVisible();
  // 断定せず「推定」であることと、発行元を示す。
  await expect(advisory).toContainText('投稿した人自身の申告でも');
  await expect(dialog.getByTestId(`post-advisory-issuer-${OBJECT_ID}`)).toContainText(
    'index.kukuri.example'
  );
  await expect(advisory).toContainText('性的表現の可能性');
  await expect(advisory).toContainText('84');
  await capture(page, 'ja-dark-1400-advisory-gated');
});

test('the advisory placeholder opens an appeal addressed to the issuing node', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'dark', advisoryLabeled: true });
  const explore = await runExploreSearch(page);

  // キーボードからも詳細 dialog と申し立てへ到達できる(カード全体の開く操作に飲み込まれない)。
  const placeholder = explore.getByTestId(`media-adult-gated-${OBJECT_ID}`);
  await placeholder.focus();
  await page.keyboard.press('Enter');
  const appeal = page.getByTestId(`post-advisory-appeal-${OBJECT_ID}`);
  await appeal.focus();
  await expect(appeal).toBeFocused();
  await page.keyboard.press('Enter');

  const dialog = page.getByRole('dialog', { name: 'リスク判定への異議申し立て' });
  await expect(dialog).toBeVisible();
  await capture(page, 'ja-dark-1400-advisory-appeal');
  await page.keyboard.press('Escape');
  await expect(placeholder).toBeFocused();
});

test('the advisory explanation stays inside a narrow column', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'light', advisoryLabeled: true });
  const explore = await runExploreSearch(page);

  await expect(explore.getByTestId(`media-adult-gated-${OBJECT_ID}`)).toBeVisible();
  const overflow = await explore.evaluate((root) => root.scrollWidth - root.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  await capture(page, 'ja-light-390-advisory-gated');
});
