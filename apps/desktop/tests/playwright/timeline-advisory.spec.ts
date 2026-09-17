import { expect, test, type Page } from '@playwright/test';

import {
  TIMELINE_ADVISORY_OBJECT_ID as OBJECT_ID,
  seedTimelineAdvisory,
} from './timeline-advisory-fixture';

// #1056: タイムラインで、採用ノードの推定が付いた投稿が「見つける」と同じ代替表示になること、
// 照会中はスケルトン(代替表示とは別)になること、設定で採用を切り替えられることを実ブラウザで確認する。
// 視覚回帰 baseline は増やさず、記録用の画像だけを環境変数指定時に書き出す。変更前の撮影では
// 新しい挙動の検証を行わない(KUKURI_1056_SHOT_PREFIX=before)。

const SHOT_DIR = '../../docs/ui-reviews/assets/1056';
const SHOT_PREFIX = process.env.KUKURI_1056_SHOT_PREFIX ?? 'after';
const VERIFY = SHOT_PREFIX === 'after';

async function capture(page: Page, name: string) {
  if (!process.env.KUKURI_1056_SHOTS) return;
  await page.screenshot({ path: `${SHOT_DIR}/${SHOT_PREFIX}-${name}.png` });
}

test('advisory-labeled timeline posts are gated and explain the issuing node', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'dark', lookup: 'advisory' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  const card = page.locator(`[data-post-object-id="${OBJECT_ID}"]`).first();
  await expect(card).toBeVisible();
  if (VERIFY) {
    const placeholder = card.getByTestId(`media-adult-gated-${OBJECT_ID}`);
    await expect(placeholder).toBeVisible();
    await expect(card.getByTestId(`media-preview-${OBJECT_ID}`)).toHaveCount(0);
    // #1108: 説明は詳細 dialog に置く。
    await placeholder.click();
    const dialog = page.getByRole('dialog', { name: 'コミュニティノードによる推定' });
    await expect(dialog.getByTestId(`post-advisory-gated-${OBJECT_ID}`)).toBeVisible();
    const appeal = dialog.getByTestId(`post-advisory-appeal-${OBJECT_ID}`);
    await appeal.focus();
    await expect(appeal).toBeFocused();
    await page.keyboard.press('Escape');
  } else {
    await page.waitForTimeout(1500);
  }
  await capture(page, 'ja-dark-1400-timeline-gated');
});

test('the placeholder stays narrow-safe in the light theme', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'light', lookup: 'advisory' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  const card = page.locator(`[data-post-object-id="${OBJECT_ID}"]`).first();
  await expect(card).toBeVisible();
  if (VERIFY) {
    await expect(card.getByTestId(`media-adult-gated-${OBJECT_ID}`)).toBeVisible();
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    );
    expect(overflow).toBeLessThanOrEqual(0);
  } else {
    await page.waitForTimeout(1500);
  }
  await capture(page, 'ja-light-390-timeline-gated');
});

test('pending lookups show a skeleton that differs from the placeholder', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'dark', lookup: 'pending' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  const card = page.locator(`[data-post-object-id="${OBJECT_ID}"]`).first();
  await expect(card).toBeVisible();
  if (!VERIFY) return;
  const pending = card.getByTestId(`media-advisory-pending-${OBJECT_ID}`);
  await expect(pending).toBeVisible();
  await expect(pending).toHaveAttribute('aria-busy', 'true');
  await expect(card.getByTestId(`media-adult-gated-${OBJECT_ID}`)).toHaveCount(0);
  await expect(card.getByTestId(`media-preview-${OBJECT_ID}`)).toHaveCount(0);
  await capture(page, 'ja-dark-1400-timeline-pending');
});

test('the adoption toggle is shown per node in Community Node settings', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'dark', lookup: 'advisory' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=community-node');
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  if (!VERIFY) return;
  const toggle = dialog.getByRole('checkbox', { name: 'このノードの成人向け表現の推定を使う' });
  await toggle.evaluate((element) => element.scrollIntoView({ block: 'center' }));
  await expect(toggle).toBeChecked();
  await capture(page, 'ja-dark-1400-settings-adoption');
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect(dialog).toContainText('このノードへは推定を確認せず、推定も使いません。');
});
