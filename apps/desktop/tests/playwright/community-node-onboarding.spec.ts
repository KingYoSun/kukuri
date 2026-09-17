import { expect, test } from '@playwright/test';
import { communityNodeCalls, seedUnconsentedCommunityNodes } from './community-node-fixture';

const copy = {
  en: { title: 'What is a community node?', review: 'Review terms', accept: 'Accept', later: 'Later', notAccepted: 'Not accepted yet' },
  ja: { title: 'コミュニティノードとは？', review: '規約を確認する', accept: '同意する', later: 'あとで', notAccepted: '未同意' },
  'zh-CN': { title: '什么是社区节点？', review: '查看条款', accept: '接受', later: '稍后', notAccepted: '尚未接受' },
};

for (const locale of ['ja', 'en', 'zh-CN'] as const) {
  for (const theme of ['dark', 'light']) {
    for (const width of [1280, 390]) {
      test(`community node consent ${locale} ${theme} ${width}`, async ({ page }, testInfo) => {
        await page.setViewportSize({ width, height: 800 });
        await seedUnconsentedCommunityNodes(page, { locale, theme });
        await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
        const intro = page.getByRole('dialog', { name: copy[locale].title });
        await expect(intro).toBeVisible();
        await expect(intro.getByRole('heading', { name: copy[locale].title })).toBeFocused();
        expect(await communityNodeCalls(page)).toEqual([]);
        const geometry = await intro.evaluate((node) => ({
          scroll: node.scrollWidth, width: node.clientWidth,
          left: node.getBoundingClientRect().left, right: node.getBoundingClientRect().right,
        }));
        expect(geometry.scroll).toBeLessThanOrEqual(geometry.width + 1);
        expect(geometry.left).toBeGreaterThanOrEqual(0);
        expect(geometry.right).toBeLessThanOrEqual(width + 1);
        await testInfo.attach('community-node-introduction', { body: await page.screenshot(), contentType: 'image/png' });
        await intro.getByRole('button', { name: copy[locale].review }).click();
        const policies = page.getByRole('dialog');
        await expect(policies).toHaveCount(1);
        // #1106: 文書は見出しだけで一覧され、同意状況が見える。本文は展開して読む。
        const terms = policies.getByRole('button', { name: 'Terms of Service' });
        await expect(terms).toHaveAttribute('aria-expanded', 'false');
        await expect(policies.getByRole('button', { name: 'Privacy Policy' })).toHaveAttribute('aria-expanded', 'false');
        await expect(policies.getByText(copy[locale].notAccepted, { exact: true })).toHaveCount(2);
        await expect(policies.getByText('You must follow the community node terms of service.')).toHaveCount(0);
        await expect(policies.getByRole('heading').first()).toBeFocused();
        await terms.click();
        const body = policies.getByRole('region', { name: 'Terms of Service' });
        await expect(body.getByText('You must follow the community node terms of service.')).toBeVisible();
        await expect(body.getByRole('heading', { name: 'Scope' })).toBeVisible();
        await expect(body.getByRole('link', { name: 'operator support' })).toHaveAttribute('href', 'https://example.com/support');
        await expect(body).not.toContainText('## Scope');
        const content = await policies.evaluate((node) => ({ scroll: node.scrollWidth, width: node.clientWidth }));
        expect(content.scroll).toBeLessThanOrEqual(content.width + 1);
        expect(await communityNodeCalls(page)).toEqual(['policies:https://first.example']);
        await testInfo.attach('community-node-policies', { body: await page.screenshot(), contentType: 'image/png' });
        await policies.getByRole('button', { name: copy[locale].accept, exact: true }).click();
        await expect(policies).not.toBeVisible();
        await expect(page.getByTestId('community-index-explore').getByRole('textbox')).toBeVisible();
        expect(await communityNodeCalls(page)).toEqual(['policies:https://first.example', 'accept:https://first.example']);
      });
    }
  }
}

test('community node policy failure can retry and Escape does not accept', async ({ page }) => {
  await seedUnconsentedCommunityNodes(page, { failPoliciesOnce: true });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  await page.getByRole('dialog').getByRole('button', { name: 'Review terms' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByRole('button', { name: 'Accept', exact: true })).toBeDisabled();
  await dialog.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Accept', exact: true })).toBeEnabled();
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  expect(await communityNodeCalls(page)).toEqual(['policies:https://first.example', 'policies:https://first.example']);
  await expect(page.getByTestId('community-index-explore').getByRole('button', { name: 'Review terms' })).toBeVisible();
});

test('community node explanation reflows at 200 percent zoom and closes with the keyboard', async ({ page }) => {
  await seedUnconsentedCommunityNodes(page, { locale: 'ja' });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  await page.evaluate(() => { document.documentElement.style.zoom = '2'; });
  const dialog = page.getByRole('dialog', { name: copy.ja.title });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: copy.ja.later }).focus();
  await page.keyboard.press('Enter');
  await expect(dialog).not.toBeVisible();
  expect(await communityNodeCalls(page)).toEqual([]);
});

test('community node policies expand and collapse from the keyboard without accepting', async ({ page }) => {
  await seedUnconsentedCommunityNodes(page, { locale: 'en' });
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  await page.getByRole('dialog').getByRole('button', { name: 'Review terms' }).click();
  const dialog = page.getByRole('dialog');
  const terms = dialog.getByRole('button', { name: 'Terms of Service' });
  await expect(terms).toBeVisible();
  await page.keyboard.press('Tab');
  await expect(terms).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(terms).toHaveAttribute('aria-expanded', 'true');
  await expect(dialog.getByRole('region', { name: 'Terms of Service' }).getByRole('heading', { name: 'Scope' })).toBeVisible();
  await page.keyboard.press('Space');
  await expect(terms).toHaveAttribute('aria-expanded', 'false');
  await expect(dialog.getByRole('region')).toHaveCount(0);
  expect(await communityNodeCalls(page)).toEqual(['policies:https://first.example']);
});
