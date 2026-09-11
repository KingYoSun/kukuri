// #960: 見つけるの検索成功 0 件で、検索先・照合対象・理由・次の行動が Column 幅に収まり、
// 各導線が既存の設定 / タイムラインへ到達することを 3 locale × 2 theme × 2 幅で確認する。
import { expect, test } from '@playwright/test';

import en from '../../src/i18n/locales/en/shell.json' with { type: 'json' };
import ja from '../../src/i18n/locales/ja/shell.json' with { type: 'json' };
import zh from '../../src/i18n/locales/zh-CN/shell.json' with { type: 'json' };

import { expectIndexContentContained, indexLayoutCalls, indexStatusCalls, seedIndexLayout } from './community-index-layout-fixture';

const NO_MATCH_QUERY = 'layout-no-matching-post';

for (const [locale, copy] of [['en', en], ['ja', ja], ['zh-CN', zh]] as const) {
  for (const [theme, width] of [['dark', 1280], ['light', 390]] as const) {
    test(`empty search guidance ${locale} ${theme} ${width}`, async ({ page }) => {
      await seedIndexLayout(page, { locale, theme, threeColumns: width >= 1280 });
      await page.setViewportSize({ width, height: 900 });
      await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
      const workspace = page.getByTestId('community-index-explore');
      const labels = copy.communityIndex;
      await workspace.getByRole('textbox', { name: labels.queryLabel }).fill(NO_MATCH_QUERY);
      await workspace.getByRole('button', { name: labels.run, exact: true }).click();

      const empty = workspace.getByTestId('community-index-empty-state');
      await expect(empty).toBeVisible();
      await expect(empty).toHaveAttribute('role', 'status');
      await expect(empty.getByText(labels.empty, { exact: true })).toBeVisible();
      await expect(empty.getByText(`${labels.nodeLabel}: https://api.kukuri.app`)).toBeVisible();
      await expect(empty.getByText(labels.emptyState.scope.explore, { exact: true })).toBeVisible();
      await expect(empty.getByText(labels.emptyState.matchesPostText, { exact: true })).toBeVisible();
      await expect(empty.getByText(labels.emptyState.reasons.notIndexedYet, { exact: true })).toBeVisible();
      await expect(empty.getByText(labels.emptyState.reasons.outsideNodeScope, { exact: true })).toBeVisible();
      await expect(empty.getByText(labels.emptyState.exploreRequestHint, { exact: true })).toBeVisible();
      // #975: 横断検索は同じノードへ自分の申請一覧だけを読み(所属証明なし)、確定した「申請なし」を示す。
      await expect(empty.getByText(labels.emptyState.indexStatus.ownRequests.none, { exact: true })).toBeVisible();
      expect(await indexStatusCalls(page)).toEqual([
        { baseUrl: 'https://api.kukuri.app', scopeKind: null, withSecretConfirmation: false },
      ]);
      await expect(empty.getByRole('button', { name: labels.emptyState.actions.requestIndexing, exact: true })).toHaveCount(0);
      await expect(empty.getByRole('button', { name: labels.emptyState.actions.openAuthor, exact: true })).toHaveCount(0);
      await expectIndexContentContained(workspace);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

      // 再検索は同じ検索語を同じノードへ送り直す。
      await empty.getByRole('button', { name: labels.emptyState.actions.retrySearch, exact: true }).click();
      await expect(empty).toBeVisible();
      expect((await indexLayoutCalls(page)).filter((call) => call.method === 'searchCommunityNodeIndex')).toHaveLength(2);
      await expect(empty.getByText(labels.emptyState.indexStatus.ownRequests.none, { exact: true })).toBeVisible();
      expect(await indexStatusCalls(page)).toHaveLength(2);

      // 接続診断は既存の設定 section へ移り、閉じると検索語と空状態を保つ。
      await empty.getByRole('button', { name: labels.emptyState.actions.openConnectivity, exact: true }).click();
      const drawer = page.getByRole('dialog');
      await expect(drawer.getByTestId('settings-section-connectivity')).toHaveAttribute('aria-current', 'location');
      await expect(page).toHaveURL(/settings=connectivity/);
      await page.keyboard.press('Escape');
      await expect(drawer).toHaveCount(0);
      await expect(workspace.getByRole('textbox', { name: labels.queryLabel })).toHaveValue(NO_MATCH_QUERY);
      await expect(empty).toBeVisible();

      // 既存のコミュニティノード設定導線も同じ空状態から到達できる。
      await empty.getByRole('button', { name: copy.workspace.communityNodeUnavailableAction, exact: true }).click();
      await expect(page.getByRole('dialog').getByTestId('settings-section-community-node')).toHaveAttribute('aria-current', 'location');
      await page.keyboard.press('Escape');
      await expect(page.getByRole('dialog')).toHaveCount(0);

      // タイムラインへ戻る導線は timeline route へ移る。
      await empty.getByRole('button', { name: labels.emptyState.actions.openTimeline, exact: true }).click();
      await expect(page).toHaveURL(/#\/timeline/);
    });
  }
}

test('a user ID query offers to open the user from the empty state', async ({ page }) => {
  await seedIndexLayout(page, { locale: 'en', theme: 'dark' });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  const workspace = page.getByTestId('community-index-explore');
  const pubkey = 'c'.repeat(64);
  await workspace.getByRole('textbox', { name: en.communityIndex.queryLabel }).fill(pubkey);
  await workspace.getByRole('button', { name: en.communityIndex.run, exact: true }).click();
  const empty = workspace.getByTestId('community-index-empty-state');
  await expect(empty.getByText(en.communityIndex.emptyState.reasons.pubkeyQuery, { exact: true })).toBeVisible();
  await expect(empty.getByText(en.communityIndex.emptyState.matchesPostText, { exact: true })).toHaveCount(0);
  await empty.getByRole('button', { name: en.communityIndex.emptyState.actions.openAuthor, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`authorPubkey=${pubkey}`));
});
