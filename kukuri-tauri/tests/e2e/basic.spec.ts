import { expect, browser, $ } from '@wdio/globals';

describe('Kukuri E2E Tests', () => {
  it('should load the application', async () => {
    // Tauriアプリケーションは既に起動しているため、現在のページを使用
    // browser.url()を使わずに現在のページを確認
    
    const body = await $('body');
    await expect(body).toBeDisplayed();
  });

  it('should display the main application container', async () => {
    const appContainer = await $('#root');
    await expect(appContainer).toExist();
  });

  it('should check page title', async () => {
    const title = await browser.getTitle();
    expect(title).toBeDefined();
    console.log('Page title:', title);
  });

  it('should handle app elements', async () => {
    // Tauriアプリケーション内の要素を確認
    const elements = await $$('div');
    expect(elements.length).toBeGreaterThan(0);
  });
});