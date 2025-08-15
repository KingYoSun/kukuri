import { expect, browser, $ } from '@wdio/globals';
import { logBrowserState, waitForTauriApp } from './helpers/debug';

describe('Kukuri E2E Tests', () => {
  before(async () => {
    // Tauriアプリケーションの起動を待つ
    await waitForTauriApp(5000);

    // 現在のブラウザ状態をログ出力
    await logBrowserState();
  });

  it('should load the application', async () => {
    // Tauriアプリケーションの起動を確認
    await browser.waitUntil(
      async () => {
        try {
          const body = await $('body');
          return await body.isExisting();
        } catch {
          return false;
        }
      },
      {
        timeout: 10000,
        timeoutMsg: 'Application failed to load',
        interval: 1000
      }
    );

    const body = await $('body');
    await expect(body).toBeDisplayed();
  });

  it('should display the main application container', async () => {
    // #root要素が存在するまで待機
    await browser.waitUntil(
      async () => {
        try {
          const appContainer = await $('#root');
          return await appContainer.isExisting();
        } catch {
          return false;
        }
      },
      {
        timeout: 10000,
        timeoutMsg: '#root element not found',
        interval: 1000
      }
    );

    const appContainer = await $('#root');
    await expect(appContainer).toExist();

    // Reactアプリがレンダリングされていることを確認
    await browser.waitUntil(
      async () => {
        try {
          const childCount = await browser.execute(() => {
            const root = document.getElementById('root');
            return root ? root.children.length : 0;
          });
          return childCount > 0;
        } catch {
          return false;
        }
      },
      { timeout: 10000, timeoutMsg: 'React app failed to render' }
    );
  });

  it('should check page title', async () => {
    const title = await browser.getTitle();
    expect(title).toBeDefined();
    // console.log('Page title:', title);  // デバッグ時のみ有効化
  });

  it('should handle app elements', async () => {
    // 要素が存在するまで待機
    await browser.waitUntil(
      async () => {
        try {
          const elements = await $$('div');
          return elements.length > 0;
        } catch {
          return false;
        }
      },
      {
        timeout: 10000,
        timeoutMsg: 'No div elements found',
        interval: 1000
      }
    );

    // Tauriアプリケーション内の要素を確認
    const elements = await $$('div');
    expect(elements.length).toBeGreaterThan(0);

    // ホームページの要素を確認
    const homePage = await $('[data-testid="home-page"]');
    if (await homePage.isExisting()) {
      await expect(homePage).toBeDisplayed();
    }
  });
});
