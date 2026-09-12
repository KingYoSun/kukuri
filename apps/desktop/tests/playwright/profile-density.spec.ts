import { expect, test } from '@playwright/test';
import { seedProfileConnections } from './profile-connections-fixture';

for (const width of [1280, 1024, 390]) {
  test(`profile actions and compact spacing at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 840 });
    await seedProfileConnections(page);
    await page.goto('/#/profile?topic=kukuri%3Atopic%3Ageneral&profileMode=connections&connectionsView=blocking');
    const row = page.getByTestId('profile-connection-identifier-target');
    await expect(row).toBeVisible();
    await expect(row.getByText('browser peer', { exact: true })).toHaveCount(1);
    const primary = row.getByRole('button', { name: 'ブロック解除', exact: true });
    await expect(primary).toBeVisible();
    const menu = row.getByRole('button', { name: 'browser peerの操作', exact: true });
    await menu.click();
    await expect(page.getByRole('menuitem')).toHaveCount(2);
    await expect(page.getByRole('menuitem', { name: 'フォロー解除', exact: true })).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(page.getByRole('menuitem', { name: 'ミュート', exact: true })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(menu).toBeFocused();
    await expect(page.getByRole('menu')).toHaveCount(0);

    const layout = await row.evaluate((element) => {
      const tabs = document.querySelector('.shell-profile-connections-tabs')!;
      const button = tabs.querySelector('button')!;
      const info = element.querySelector('.author-detail-copy-stack')!;
      const primary = element.querySelector('.profile-connection-primary-action')!;
      const badges = element.querySelector('.profile-connection-badges')!;
      return {
        listGap: element.parentElement!.parentElement!.getBoundingClientRect().top - tabs.getBoundingClientRect().bottom,
        tabGap: parseFloat(getComputedStyle(tabs).gap),
        paddingY: parseFloat(getComputedStyle(button).paddingTop),
        paddingX: parseFloat(getComputedStyle(button).paddingLeft),
        targetHeight: button.getBoundingClientRect().height,
        bioGap: parseFloat(getComputedStyle(info).gap),
        primaryRight: primary.getBoundingClientRect().left >= info.getBoundingClientRect().right,
        badgesAbove: badges.getBoundingClientRect().bottom <= info.getBoundingClientRect().top,
      };
    });
    expect(layout.listGap).toBeGreaterThanOrEqual(8);
    expect(layout.tabGap).toBeGreaterThan(0);
    expect(layout.tabGap).toBeLessThanOrEqual(6);
    expect(layout.paddingY).toBeGreaterThan(0);
    expect(layout.paddingY).toBeLessThanOrEqual(8);
    expect(layout.paddingX).toBeLessThanOrEqual(8);
    expect(layout.targetHeight).toBeGreaterThanOrEqual(32);
    expect(layout.bioGap).toBeGreaterThan(0);
    expect(layout.bioGap).toBeLessThanOrEqual(6);
    expect(layout.primaryRight).toBe(true);
    expect(layout.badgesAbove).toBe(true);

    const columnGutters = await page.locator('.shell-column-body > .shell-main-stack').evaluateAll((stacks) => stacks.map((stack) => {
      const parent = stack.parentElement!;
      const outer = parent.getBoundingClientRect(), inner = stack.getBoundingClientRect();
      return { left: inner.left - outer.left - parent.clientLeft,
        right: outer.left + parent.clientLeft + parent.clientWidth - inner.right };
    }));
    for (const gutter of columnGutters) {
      expect(gutter.left).toBeGreaterThan(0);
      expect(Math.abs(gutter.left - gutter.right)).toBeLessThanOrEqual(1);
    }
    const postSpacing = await page.locator('[data-post-object-id]').first().evaluate((post) => {
      const meta = post.querySelector('.post-meta')!, body = post.querySelector('.post-body')!;
      return { padding: parseFloat(getComputedStyle(post).paddingTop),
        gap: parseFloat(getComputedStyle(meta).gap),
        toBody: body.getBoundingClientRect().top - meta.getBoundingClientRect().bottom };
    });
    expect(postSpacing.padding).toBe(8);
    expect(postSpacing.gap).toBe(4);
    expect(postSpacing.toBody).toBeGreaterThan(0);
    expect(postSpacing.toBody).toBeLessThanOrEqual(8);

    await primary.click();
    await expect(page.getByText('ブロック中のユーザーはいません。')).toBeVisible();
    await page.getByRole('button', { name: 'プロフィールに戻る', exact: true }).click();
    await expect(page.getByRole('button', { name: 'ブロック中 0人', exact: true })).toBeVisible();
  });
}
