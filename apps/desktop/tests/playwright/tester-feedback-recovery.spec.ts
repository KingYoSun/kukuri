import { expect, test, type Page } from '@playwright/test';
import { seedFeedback } from './tester-feedback-fixture';

const copy = {
  en: { title: 'Send feedback', settings: 'Open Community Node settings', closeSettings: 'Close settings',
    reason: 'This node does not offer feedback intake.', send: 'Send', refresh: 'Refresh' },
  ja: { title: 'フィードバックを送る', settings: 'コミュニティノード設定を開く', closeSettings: '設定を閉じる',
    reason: 'このノードはフィードバック受付を提供していません。', send: '送信する', refresh: '更新' },
  'zh-CN': { title: '发送反馈', settings: '打开社区节点设置', closeSettings: '关闭设置',
    reason: '此节点不提供反馈接收功能。', send: '发送', refresh: '刷新' },
};


async function calls(page: Page) {
  return page.evaluate(() => (window as unknown as { __feedbackFixture: { calls: string[] } }).__feedbackFixture.calls);
}

for (const locale of ['ja', 'en', 'zh-CN'] as const) {
  for (const theme of ['dark', 'light']) {
    for (const viewport of [{ width: 1280, height: 800 }, { width: 390, height: 844 }]) {
      test(`feedback recovery ${locale} ${theme} ${viewport.width}`, async ({ page }, testInfo) => {
        await page.setViewportSize(viewport);
        await seedFeedback(page, locale, theme);
        await page.goto('/#/timeline');
        const trigger = page.getByTestId('tester-feedback-trigger');
        await trigger.click();
        const dialog = page.getByRole('dialog', { name: copy[locale].title });
        await expect(dialog.getByText(copy[locale].reason, { exact: false })).toBeVisible();
        const settingsButton = dialog.getByRole('button', { name: copy[locale].settings });
        await expect(settingsButton).toBeInViewport();
        await expect(dialog.getByRole('button', { name: copy[locale].send, exact: true })).toBeDisabled();
        const bounds = await dialog.boundingBox();
        expect(bounds!.x).toBeGreaterThanOrEqual(0);
        expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(viewport.width + 1);
        expect(await dialog.evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
        await testInfo.attach('feedback-unavailable', { body: await dialog.screenshot(), contentType: 'image/png' });
        if (viewport.width === 390) {
          await settingsButton.focus();
          await page.keyboard.press('Enter');
        } else await settingsButton.click();
        const settings = page.getByRole('dialog');
        await expect(settings).toHaveCount(1);
        await expect(settings.getByTestId('settings-section-community-node')).toHaveAttribute('aria-current', 'location');
        await expect(settings.getByRole('button', { name: copy[locale].closeSettings })).toBeFocused();
        await settings.getByRole('button', { name: copy[locale].closeSettings }).click();
        await expect(settings).toHaveCount(0);
        await expect(page.getByTestId('control-center-trigger')).toBeFocused();
        await trigger.click();
        await expect(dialog.getByText(copy[locale].reason, { exact: false })).toBeVisible();
        await page.keyboard.press('Escape');
        await expect(dialog).toHaveCount(0);
        expect(await calls(page)).toEqual([]);
      });
    }
  }
}

test('feedback reflows at 200 percent zoom without hiding its recovery action', async ({ page }) => {
  await seedFeedback(page, 'ja', 'dark');
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/timeline');
  await page.evaluate(() => { document.documentElement.style.zoom = '2'; });
  await page.getByTestId('tester-feedback-trigger').click();
  const dialog = page.getByRole('dialog', { name: copy.ja.title });
  const button = dialog.getByRole('button', { name: copy.ja.settings });
  await button.scrollIntoViewIfNeeded();
  await expect(button).toBeInViewport();
  expect(await dialog.evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
  await button.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('settings-section-community-node')).toHaveAttribute('aria-current', 'location');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(await calls(page)).toEqual([]);
});

test('settings refresh updates feedback availability before reopening without automatically sending', async ({ page }) => {
  await seedFeedback(page, 'en', 'dark');
  await page.goto('/#/timeline');
  await page.getByTestId('tester-feedback-trigger').click();
  await page.getByRole('button', { name: copy.en.settings }).click();
  await page.evaluate(() => {
    (window as unknown as { __feedbackFixture: { supported: boolean } }).__feedbackFixture.supported = true;
  });
  await page.getByRole('dialog').getByRole('button', { name: copy.en.refresh, exact: true }).click();
  await page.getByRole('button', { name: copy.en.closeSettings }).click();
  await page.getByTestId('tester-feedback-trigger').click();
  const dialog = page.getByRole('dialog', { name: copy.en.title });
  await expect(dialog.getByRole('combobox')).toHaveValue('https://feedback.example');
  await expect(dialog.getByText(copy.en.reason, { exact: false })).toHaveCount(0);
  expect(await calls(page)).toEqual([]);
});
