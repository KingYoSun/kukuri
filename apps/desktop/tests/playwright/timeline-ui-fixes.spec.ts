import { expect, test } from '@playwright/test';
import { installReplyLayoutFixture } from './reply-layout-fixture';

for (const locale of ['ja', 'en', 'zh-CN']) {
  for (const width of [390, 759, 760, 1440]) {
    test(`immediate reply parent and complete date fit ${locale} at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.addInitScript(locale => localStorage.setItem('kukuri.desktop.locale', locale), locale);
      await page.addInitScript(installReplyLayoutFixture);
      await page.goto('/');
      const group = page.locator('.post-reply-group').first();
      const parent = group.locator('.post-reply-context');
      const child = group.locator('article');
      await expect(parent).toContainText('GrokTester');
      await expect(child).toContainText('CliPeerA');
      const bounds = await group.evaluate(element => {
        const parent = element.querySelector('.post-reply-context')!.getBoundingClientRect();
        const child = element.querySelector('article')!.getBoundingClientRect();
        const time = element.querySelector('time')!.getBoundingClientRect();
        const excerpt = element.querySelector('.post-reply-context-body')!;
        return { parentBottom: parent.bottom, childTop: child.top, childRight: child.right,
          timeRight: time.right, timeLeft: time.left, childLeft: child.left,
          excerptHeight: excerpt.getBoundingClientRect().height, lineHeight: parseFloat(getComputedStyle(excerpt).lineHeight),
          overflow: element.scrollWidth - element.clientWidth };
      });
      expect(bounds.parentBottom).toBeLessThan(bounds.childTop);
      expect(bounds.timeRight).toBeLessThanOrEqual(bounds.childRight);
      expect(bounds.timeLeft).toBeGreaterThanOrEqual(bounds.childLeft);
      expect(bounds.excerptHeight).toBeLessThanOrEqual(bounds.lineHeight * 2 + 1);
      expect(bounds.overflow).toBeLessThanOrEqual(1);
      await expect(child.locator('time')).toContainText('2026');
      await expect(child.locator('time')).toContainText(/32:01/);
    });
  }
}

test('column bottoms leave room above the horizontal scrollbar', async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 768 });
  await page.goto('/');
  await expect(page.locator('.shell-column-surface').first()).toBeVisible();
  const geometry = await page.locator('.shell-column-canvas').evaluate((canvas) => {
    const bounds = canvas.getBoundingClientRect();
    const columns = Array.from(canvas.querySelectorAll('.shell-column-surface'));
    return {
      gaps: columns.map(column => bounds.top + canvas.clientTop + canvas.clientHeight - column.getBoundingClientRect().bottom),
      required: parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.75,
      documentOverflow: document.documentElement.scrollHeight - window.innerHeight,
    };
  });
  for (const gap of geometry.gaps) expect(gap).toBeGreaterThanOrEqual(geometry.required - 1);
  expect(geometry.documentOverflow).toBeLessThanOrEqual(1);
});

test('control center hides the account trigger and restores focus on close', async ({ page }) => {
  await page.goto('/');
  const account = page.getByTestId('account-menu-trigger');
  const trigger = page.getByTestId('control-center-trigger');
  await expect(account).toBeVisible();
  const box = (await account.boundingBox())!;
  await trigger.click();
  await expect(page.locator('#shell-control-center')).toBeVisible();
  await expect(account).toBeHidden();
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await expect(page.getByRole('menu')).toHaveCount(0);
  await page.keyboard.press('Tab');
  await expect(account).not.toBeFocused();
  await page.keyboard.press('Escape');
  await expect(account).toBeVisible();
  await expect(trigger).toBeFocused();
  await account.click();
  await expect(page.getByRole('menu')).toBeVisible();
});
