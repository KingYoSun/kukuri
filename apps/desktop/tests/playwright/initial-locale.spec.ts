import { expect, test } from '@playwright/test';
import { appConsentCalls, seedAppConsent } from './app-consent-fixture';

test('fresh Japanese OS locale controls the first consent render even with an English navigator', async ({ page }) => {
  await seedAppConsent(page, { locale: null, systemLocales: ['ja_JP.UTF-8'] });
  await page.addInitScript(() => {
    const observer = new MutationObserver(() => {
      const heading = document.querySelector('.app-consent-panel h1');
      if (!heading) return;
      document.documentElement.dataset.firstConsentTitle = heading.textContent ?? '';
      observer.disconnect();
    });
    observer.observe(document, { childList:true, subtree:true });
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('ご利用の前に');
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await expect(page.locator('html')).toHaveAttribute('data-first-consent-title', 'ご利用の前に');
  await expect(page.getByRole('combobox', { name: '言語', exact: true })).toHaveValue('ja');
  expect(await page.evaluate(() => localStorage.getItem('kukuri.desktop.locale'))).toBe('ja');
  expect(await appConsentCalls(page)).toEqual([]);
});

test('a supported saved language overrides the OS on restart', async ({ page }) => {
  await seedAppConsent(page, { locale: 'en', systemLocales:['ja-JP'] });
  await page.goto('/');
  await expect(page.getByRole('heading', { level:1 })).toHaveText('Before you continue');
  await expect(page.getByRole('combobox', { name:'Language' })).toHaveValue('en');
  expect(await page.evaluate(() => (window as unknown as {__appConsentCalls:{command:string}[]})
    .__appConsentCalls.some(call=>call.command==='get_system_locales'))).toBe(false);
});

test('a fresh browser uses navigator without depending on native IPC', async ({ browser }) => {
  const context = await browser.newContext({ locale:'ja-JP' });
  const page = await context.newPage();
  try {
    await page.goto('/');
    await expect(page.getByTestId('control-center-trigger')).toBeVisible();
    await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
    expect(await page.evaluate(() => localStorage.getItem('kukuri.desktop.locale'))).toBe('ja');
  } finally { await context.close(); }
});

test('language is selectable before consent and the saved choice survives reload', async ({ page }) => {
  await seedAppConsent(page, { locale: null, systemLocales: ['en-US'] });
  await page.goto('/');
  const language = page.getByRole('combobox', { name: /Language/ });
  await expect(language).toBeVisible();
  await expect(language.getByRole('option', { name: '日本語', exact: true })).toHaveCount(1);
  await language.selectOption('ja');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('ご利用の前に');
  await expect(page.locator('html')).toHaveAttribute('lang', 'ja');
  await expect(page.getByRole('checkbox')).not.toBeChecked();
  expect(await appConsentCalls(page)).toEqual([]);
  await page.reload();
  await expect(page.getByRole('combobox', { name: '言語', exact: true })).toHaveValue('ja');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('ご利用の前に');
});
