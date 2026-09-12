import type { Page } from '@playwright/test';

export async function seedProfileConnections(page: Page, locale = 'ja', theme = 'dark') {
  await page.addInitScript(({ locale, theme }) => {
    localStorage.setItem('kukuri.desktop.locale', locale);
    localStorage.setItem('kukuri.desktop.theme', theme);
    let api: typeof window.__KUKURI_DESKTOP__;
    Object.defineProperty(window, '__KUKURI_DESKTOP__', {
      configurable: true, get: () => api,
      set: (value: NonNullable<typeof api>) => {
        api = value;
        // The mock updates its projection synchronously before returning the Promise.
        void value.blockAuthor('b'.repeat(64));
      },
    });
  }, { locale, theme });
}
