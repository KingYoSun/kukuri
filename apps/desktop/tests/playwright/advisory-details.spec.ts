import { expect, test, type Page } from '@playwright/test';

import { OBJECT_ID as EXPLORE_OBJECT_ID, runExploreSearch, seedExploreMedia } from './community-index-media-fixture';
import {
  TIMELINE_ADVISORY_OBJECT_ID,
  TIMELINE_ADVISORY_URL,
  seedTimelineAdvisory,
} from './timeline-advisory-fixture';

// #1108: 推定による代替表示を、枠と短いラベルだけにして詳細を dialog へ移したことを実ブラウザで確認する。
// 記録用の画像は環境変数指定時だけ書き出す。変更前の撮影(KUKURI_1108_SHOT_PREFIX=before)では
// 新しい挙動の検証を行わない。

const SHOT_DIR = '../../docs/ui-reviews/assets/1108';
const SHOT_PREFIX = process.env.KUKURI_1108_SHOT_PREFIX ?? 'after';
const VERIFY = SHOT_PREFIX === 'after';

async function capture(page: Page, name: string) {
  if (!process.env.KUKURI_1108_SHOTS) return;
  await page.screenshot({ path: `${SHOT_DIR}/${SHOT_PREFIX}-${name}.png` });
}

async function documentOverflow(page: Page) {
  return page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );
}

for (const { locale, theme, width, label, title } of [
  { locale: 'ja', theme: 'dark', width: 1400, label: '成人向け画像: 詳細はクリック', title: 'コミュニティノードによる推定' },
  { locale: 'ja', theme: 'light', width: 390, label: '成人向け画像: 詳細はクリック', title: 'コミュニティノードによる推定' },
  { locale: 'en', theme: 'light', width: 1400, label: 'Adult image: click for details', title: 'Community Node estimate' },
  { locale: 'zh-CN', theme: 'dark', width: 390, label: '成人图片：点击查看详情', title: 'Community Node 的推测' },
] as const) {
  test(`timeline advisory details ${locale} ${theme} ${width}`, async ({ page }) => {
    await page.setViewportSize({ width, height: width < 600 ? 844 : 980 });
    await seedTimelineAdvisory(page, { locale, theme, lookup: 'advisory' });
    await page.goto(TIMELINE_ADVISORY_URL);
    const card = page.locator(`[data-post-object-id="${TIMELINE_ADVISORY_OBJECT_ID}"]`).first();
    await expect(card).toBeVisible();
    if (!VERIFY) {
      await page.waitForTimeout(1500);
      await capture(page, `${locale}-${theme}-${width}-timeline-list`);
      return;
    }

    // AC-1: 一覧には枠とラベルだけ。
    const placeholder = card.getByTestId(`media-adult-gated-${TIMELINE_ADVISORY_OBJECT_ID}`);
    await expect(placeholder).toHaveAccessibleName(label);
    await expect(card.getByTestId(`post-advisory-gated-${TIMELINE_ADVISORY_OBJECT_ID}`)).toHaveCount(0);
    await expect(card.locator('img, video')).toHaveCount(0);
    expect(await documentOverflow(page)).toBeLessThanOrEqual(0);
    await placeholder.scrollIntoViewIfNeeded();
    await capture(page, `${locale}-${theme}-${width}-timeline-list`);

    // AC-2: keyboard で開き、閉じると枠へ戻る。
    await placeholder.focus();
    await page.keyboard.press('Enter');
    const dialog = page.getByRole('dialog', { name: title });
    await expect(dialog.getByTestId(`post-advisory-gated-${TIMELINE_ADVISORY_OBJECT_ID}`)).toBeVisible();
    await expect(dialog.getByTestId(`post-advisory-appeal-${TIMELINE_ADVISORY_OBJECT_ID}`)).toBeVisible();
    await expect(dialog.locator('img, video')).toHaveCount(0);
    expect(await documentOverflow(page)).toBeLessThanOrEqual(0);
    await capture(page, `${locale}-${theme}-${width}-timeline-dialog`);
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);
    await expect(placeholder).toBeFocused();
  });
}

test('Explore advisory details open from the frame', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedExploreMedia(page, { locale: 'ja', theme: 'dark', advisoryLabeled: true });
  const explore = await runExploreSearch(page);
  const placeholder = explore.getByTestId(`media-adult-gated-${EXPLORE_OBJECT_ID}`);
  await expect(placeholder).toBeVisible();
  if (!VERIFY) {
    await capture(page, 'ja-dark-1400-explore-list');
    return;
  }

  await expect(placeholder).toHaveAccessibleName('成人向け画像: 詳細はクリック');
  await expect(explore.getByTestId(`post-advisory-gated-${EXPLORE_OBJECT_ID}`)).toHaveCount(0);
  await capture(page, 'ja-dark-1400-explore-list');

  await placeholder.click();
  const dialog = page.getByRole('dialog', { name: 'コミュニティノードによる推定' });
  await expect(dialog.getByTestId(`post-advisory-issuer-${EXPLORE_OBJECT_ID}`)).toContainText(
    'index.kukuri.example'
  );
  await capture(page, 'ja-dark-1400-explore-dialog');
  await dialog.getByRole('button', { name: 'ダイアログを閉じる' }).click();
  await expect(placeholder).toBeFocused();
});
