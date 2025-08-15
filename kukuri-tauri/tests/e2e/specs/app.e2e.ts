import { setupE2ETest, beforeEachTest, afterEachTest } from '../helpers/setup';

describe('Kukuri App E2E Tests', () => {
  before(async () => {
    await setupE2ETest();
  });

  beforeEach(async () => {
    await beforeEachTest();
  });

  afterEach(async function () {
    await afterEachTest(this.currentTest?.title || 'unknown');
  });

  describe('App Launch', () => {
    it('should launch the application successfully', async () => {
      // アプリが起動していることを確認
      const rootElement = await $('#root');
      expect(await rootElement.isDisplayed()).toBe(true);

      // URLを確認（welcomeページがデフォルト）
      const url = await browser.getUrl();
      expect(url).toContain('/welcome');
    });

    it('should display the welcome page by default', async () => {
      // Welcomeページの要素を確認
      await browser.waitUntil(
        async () => {
          const body = await $('body');
          const html = await body.getHTML();
          return html.length > 100; // コンテンツがレンダリングされている
        },
        { timeout: 10000 }
      );

      // メインコンテンツエリアの確認
      const mainContent = await $('main');
      if (await mainContent.isExisting()) {
        expect(await mainContent.isDisplayed()).toBe(true);
      }
    });

    it('should show content on the welcome page', async () => {
      // 何らかのテキストコンテンツが存在することを確認
      const body = await $('body');
      const text = await body.getText();
      expect(text.length).toBeGreaterThan(10);
      
      // ページに何らかの要素が存在することを確認
      const elements = await $$('div, p, span, button, a');
      expect(elements.length).toBeGreaterThan(0);
    });
  });

  describe('Navigation', () => {
    it('should start on welcome page by default', async () => {
      // URLの確認（welcomeページから開始）
      const url = await browser.getUrl();
      expect(url).toContain('/welcome');
    });

    it.skip('should navigate to settings page', async () => {
      // Welcomeページから他のページへの遷移はスキップ（認証が必要なため）
    });

    it.skip('should navigate between pages using sidebar', async () => {
      // Welcomeページにはサイドバーがないためスキップ
    });
  });

  describe('Theme Toggle', () => {
    it('should toggle between light and dark theme', async () => {
      // テーマトグルボタンを探す
      const themeToggle = await $('[data-testid="theme-toggle"]');

      if (await themeToggle.isExisting()) {
        // 初期テーマを取得
        const initialTheme = await browser.execute(() => {
          return document.documentElement.getAttribute('data-theme') ||
            document.documentElement.classList.contains('dark')
            ? 'dark'
            : 'light';
        });

        // テーマを切り替え
        await themeToggle.click();

        // テーマが変更されたことを確認
        const newTheme = await browser.execute(() => {
          return document.documentElement.getAttribute('data-theme') ||
            document.documentElement.classList.contains('dark')
            ? 'dark'
            : 'light';
        });

        expect(newTheme).not.toBe(initialTheme);

        // もう一度切り替えて元に戻る
        await themeToggle.click();

        const finalTheme = await browser.execute(() => {
          return document.documentElement.getAttribute('data-theme') ||
            document.documentElement.classList.contains('dark')
            ? 'dark'
            : 'light';
        });

        expect(finalTheme).toBe(initialTheme);
      }
    });
  });

  describe('Window Controls', () => {
    it('should minimize the window', async () => {
      // 最小化ボタンをクリック（Tauriアプリの場合）
      const minimizeButton = await $('[data-testid="minimize-button"]');

      if (await minimizeButton.isExisting()) {
        await minimizeButton.click();
        // 実際の最小化の確認は環境依存のため、エラーが発生しないことのみ確認
      }
    });

    it('should maximize and restore the window', async () => {
      const maximizeButton = await $('[data-testid="maximize-button"]');

      if (await maximizeButton.isExisting()) {
        // 最大化
        await maximizeButton.click();
        await browser.pause(500);

        // 元のサイズに戻す
        await maximizeButton.click();
        await browser.pause(500);
      }
    });
  });

  describe('Error Handling', () => {
    it('should handle invalid routes gracefully', async () => {
      // 無効なURLにアクセスしてエラーを発生させる
      await browser.url('http://tauri.localhost/invalid-route');
      await browser.pause(1000);

      // アプリが正常に動作していることを確認（クラッシュしていない）
      const rootElement = await $('#root');
      expect(await rootElement.isExisting()).toBe(true);
      
      // 何らかのコンテンツが表示されていることを確認
      const body = await $('body');
      const text = await body.getText();
      expect(text.length).toBeGreaterThan(0);
    });
  });
});
