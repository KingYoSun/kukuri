import { expect, test } from '@playwright/test';

for (const width of [1280, 1024, 390]) {
  test(`a saved post reaches the inactive profile at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 800 });
    await page.addInitScript(() => {
      window.localStorage.setItem('kukuri.desktop.locale', 'ja');
    });
    await page.goto('/');
    const timeline = page.locator('[data-column-id][aria-current="true"]');
    const profile = page.locator('[data-column-id]').filter({
      has: page.locator('.profile-overview-header'),
    });
    const timelineId = await timeline.getAttribute('data-column-id');
    const content = `プロフィール反映の確認 ${width}`;
    const count = profile.locator('.topic-diagnostic-secondary').filter({ hasText: '公開投稿' }).locator('span').last();
    await expect(count).toHaveText(/^\d+$/);
    const initialCount = Number(await count.innerText());
    await timeline.locator('.shell-column-primary-action').click();
    await timeline.getByPlaceholder('投稿を書く').fill(content);
    await timeline.locator('.shell-column-composer').getByRole('button', { name: '投稿', exact: true }).click();

    // Offscreen mobile columns are still checked for data without moving focus.
    await expect(profile.getByText(content, { exact: true })).toHaveCount(1);
    await expect(count).toHaveText(String(initialCount + 1));
    await expect(timeline).toHaveAttribute('data-column-id', timelineId!);
    await expect(profile).not.toHaveAttribute('aria-current', 'true');
    await expect(page.locator('.shell-column-composer')).toHaveCount(0);
    if (width === 390) {
      await profile.scrollIntoViewIfNeeded();
      await expect(profile.getByText(content, { exact: true })).toBeVisible();
    }
    await page.screenshot({ path: testInfo.outputPath('profile-public-post.png') });
  });
}
