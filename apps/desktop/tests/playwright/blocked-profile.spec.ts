import { expect, test, type Page } from '@playwright/test';

// #992: ブロック中の作者詳細ではフォロー／メッセージが無効になり、理由は tooltip で出る。解除で同じ画面から復帰する。
const PEER = 'b'.repeat(64);
const FOLLOW_REASON = 'ブロック中はフォローできません。先にブロックを解除してください。';
const MESSAGE_REASON = 'ブロック中はメッセージを送れません。先にブロックを解除してください。';

async function seedBlockedPeer(page: Page, unfollow: boolean) {
  await page.addInitScript(({ peer, unfollow }) => {
    localStorage.setItem('kukuri.desktop.locale', 'ja');
    localStorage.setItem('kukuri.desktop.theme', 'dark');
    let api: typeof window.__KUKURI_DESKTOP__;
    Object.defineProperty(window, '__KUKURI_DESKTOP__', {
      configurable: true, get: () => api,
      set: (value: NonNullable<typeof api>) => {
        api = value;
        void value.blockAuthor(peer);
        if (unfollow) void value.unfollowAuthor(peer);
      },
    });
  }, { peer: PEER, unfollow });
}

async function openBrowserPeerDetail(page: Page) {
  await page.goto('/');
  await page.getByText('browser mock peer post').click();
  // 新しく積まれた Thread Column の投稿者ボタン（最後尾）から作者詳細を開く。
  await page.getByRole('button', { name: 'browser peer', exact: true }).last().click();
  const detail = page.locator('.author-detail');
  await expect(detail).toBeVisible();
  return detail;
}

for (const width of [1280, 390]) {
  test(`blocked and unfollowed peer keeps follow disabled with a tooltip until unblocked at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 840 });
    await seedBlockedPeer(page, true);
    const detail = await openBrowserPeerDetail(page);

    await expect(detail.getByText('ブロック中', { exact: true })).toBeVisible();
    const follow = detail.getByRole('button', { name: 'フォロー', exact: true });
    await expect(follow).toHaveAttribute('aria-disabled', 'true');
    await expect(follow).toHaveAccessibleDescription(FOLLOW_REASON);
    await expect(detail.getByRole('button', { name: 'メッセージ', exact: true })).toHaveCount(0);

    await follow.hover();
    const tooltip = page.getByRole('tooltip');
    await expect(tooltip).toHaveText(FOLLOW_REASON);
    const box = (await tooltip.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    if (width === 1280) {
      await page.screenshot({ path: '../../docs/progress/assets/992/author-blocked-tooltip-ja-1280.png' });
    }
    // aria-disabled の要素は Playwright の actionability 検査で止まるため force で実 click を送る。
    await follow.click({ force: true });
    await expect(follow).toHaveAttribute('aria-disabled', 'true');
    // Escape は shell が detail pane を閉じる操作なので、pointer を離して tooltip を閉じる。
    await page.mouse.move(0, 0);
    await detail.getByRole('button', { name: 'ミュート', exact: true }).focus();
    await expect(page.getByRole('tooltip')).toHaveCount(0);

    // keyboard: focus だけでも理由が開く。
    await page.keyboard.press('Shift+Tab');
    await expect(follow).toBeFocused();
    await expect(page.getByRole('tooltip')).toHaveText(FOLLOW_REASON);
    await page.keyboard.press('Tab');
    await expect(page.getByRole('tooltip')).toHaveCount(0);

    await detail.getByRole('button', { name: 'ブロック解除', exact: true }).click();
    await expect(follow).not.toHaveAttribute('aria-disabled', 'true');
    await expect(detail.getByText('ブロック中', { exact: true })).toHaveCount(0);
    await expect(detail.getByRole('button', { name: 'ブロック', exact: true })).toBeVisible();
    if (width === 1280) {
      await page.mouse.move(0, 0);
      await page.screenshot({ path: '../../docs/progress/assets/992/author-unblocked-ja-1280.png' });
    }
    await follow.click();
    await expect(detail.getByRole('button', { name: 'フォロー解除', exact: true })).toBeVisible();
  });
}

test('blocked mutual peer keeps message disabled while unfollow stays enabled', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 840 });
  await seedBlockedPeer(page, false);
  const detail = await openBrowserPeerDetail(page);

  const message = detail.getByRole('button', { name: 'メッセージ', exact: true });
  await expect(message).toHaveAttribute('aria-disabled', 'true');
  await expect(message).toHaveAccessibleDescription(MESSAGE_REASON);
  await expect(detail.getByRole('button', { name: 'フォロー解除', exact: true })).not.toHaveAttribute('aria-disabled', 'true');
  await message.click({ force: true });
  await expect(page).not.toHaveURL(/#\/messages/);
  await message.hover();
  await expect(page.getByRole('tooltip')).toHaveText(MESSAGE_REASON);
  await page.mouse.move(0, 0);

  await detail.getByRole('button', { name: 'ブロック解除', exact: true }).click();
  await expect(message).not.toHaveAttribute('aria-disabled', 'true');
  await message.click();
  await expect(page).toHaveURL(/#\/messages/);
});
