import { AppHelper } from '../helpers/app';
import { setupE2ETest, beforeEachTest, afterEachTest } from '../helpers/setup';

describe('Authentication E2E Tests', () => {
  before(async () => {
    await setupE2ETest();
  });

  beforeEach(async () => {
    await beforeEachTest();
  });

  afterEach(async function () {
    await afterEachTest(this.currentTest?.title || 'unknown');
  });

  describe('Welcome Page', () => {
    it('should display welcome page with authentication options', async () => {
      // Welcomeページが表示されていることを確認
      const url = await browser.getUrl();
      expect(url).toContain('/welcome');

      // ウェルカムメッセージが表示されていることを確認
      const welcomeText = await AppHelper.findByText('kukuriへようこそ');
      await expect(welcomeText).toBeDisplayed();

      // 認証ボタンが存在することを確認
      const createAccountBtn = await AppHelper.findByText('新規アカウント作成', 'button');
      const loginBtn = await AppHelper.findByText('既存アカウントでログイン', 'button');
      
      await expect(createAccountBtn).toBeDisplayed();
      await expect(loginBtn).toBeDisplayed();
    });

    it('should create new account and redirect to profile setup', async () => {
      // 新規アカウント作成ボタンをクリック
      await AppHelper.clickButton('新規アカウント作成');

      // プロファイル設定ページへのリダイレクトを確認
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/profile-setup');
        },
        {
          timeout: 10000,
          timeoutMsg: 'Failed to redirect to profile setup page',
        }
      );

      const url = await browser.getUrl();
      expect(url).toContain('/profile-setup');
    });
  });

  describe('Main Application Access', () => {
    it('should access main application after authentication', async () => {
      // 新規アカウント作成でログイン
      await AppHelper.clickButton('新規アカウント作成');

      // プロファイル設定ページが表示されるのを待つ
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/profile-setup');
        },
        {
          timeout: 10000,
          timeoutMsg: 'Failed to redirect to profile setup page',
        }
      );

      // プロファイル設定をスキップ（または完了）してメインページへ
      // スキップボタンがある場合はクリック
      try {
        const skipBtn = await AppHelper.findByText('スキップ', 'button');
        if (await skipBtn.isExisting()) {
          await skipBtn.click();
        }
      } catch {
        // スキップボタンがない場合は、保存ボタンを探す
        try {
          const saveBtn = await AppHelper.findByText('保存', 'button');
          if (await saveBtn.isExisting()) {
            await saveBtn.click();
          }
        } catch {
          // ボタンが見つからない場合は、直接メインページへナビゲート
          const currentUrl = await browser.getUrl();
          const baseUrl = currentUrl.split('#')[0].split('?')[0];
          await browser.url(baseUrl + '#/');
        }
      }

      // メインページへのリダイレクトを待つ
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          // /profile-setupページ以外で、認証ページでもないことを確認
          return !url.includes('/profile-setup') && 
                 !url.includes('/welcome') && 
                 !url.includes('/login');
        },
        {
          timeout: 10000,
          timeoutMsg: 'Failed to access main application',
        }
      );

      // メインレイアウトが表示されていることを確認
      const rootElement = await $('#root');
      await expect(rootElement).toBeDisplayed();

      // 認証後のページであることを確認（URLが/welcomeでないこと）
      const currentUrl = await browser.getUrl();
      expect(currentUrl).not.toContain('/welcome');
    });
  });

  describe('Protected Routes', () => {
    it('should access topics page after authentication', async () => {
      // まず認証を行う
      await AppHelper.clickButton('新規アカウント作成');

      // 認証完了を待つ
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/profile-setup');
        },
        {
          timeout: 10000,
        }
      );

      // トピックページへ直接アクセス
      const topicsUrl = await browser.getUrl();
      const topicsBase = topicsUrl.split('#')[0].split('?')[0];
      await browser.url(topicsBase + '#/topics');

      // トピックページへのアクセスを確認
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/topics');
        },
        {
          timeout: 5000,
          timeoutMsg: 'Failed to access topics page',
        }
      );

      const url = await browser.getUrl();
      expect(url).toContain('/topics');
    });

    it('should access settings page after authentication', async () => {
      // まず認証を行う
      await AppHelper.clickButton('新規アカウント作成');

      // 認証完了を待つ
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/profile-setup');
        },
        {
          timeout: 10000,
        }
      );

      // 設定ページへ直接アクセス
      const settingsUrl = await browser.getUrl();
      const settingsBase = settingsUrl.split('#')[0].split('?')[0];
      await browser.url(settingsBase + '#/settings');

      // 設定ページへのアクセスを確認
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/settings');
        },
        {
          timeout: 5000,
          timeoutMsg: 'Failed to access settings page',
        }
      );

      const url = await browser.getUrl();
      expect(url).toContain('/settings');
    });
  });

  describe('Session Management', () => {
    it('should maintain authentication state across page navigation', async () => {
      // 認証を行う
      await AppHelper.clickButton('新規アカウント作成');

      // 認証完了を待つ
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/profile-setup');
        },
        {
          timeout: 10000,
        }
      );

      // ホームページへ移動
      let navUrl = await browser.getUrl();
      let navBase = navUrl.split('#')[0].split('?')[0];
      await browser.url(navBase + '#/');
      await browser.pause(1000);

      // トピックページへ移動
      navUrl = await browser.getUrl();
      navBase = navUrl.split('#')[0].split('?')[0];
      await browser.url(navBase + '#/topics');
      await browser.pause(1000);

      // 設定ページへ移動
      navUrl = await browser.getUrl();
      navBase = navUrl.split('#')[0].split('?')[0];
      await browser.url(navBase + '#/settings');
      await browser.pause(1000);

      // 各ページでwelcomeページにリダイレクトされないことを確認
      const finalUrl = await browser.getUrl();
      expect(finalUrl).not.toContain('/welcome');
      expect(finalUrl).toContain('/settings');
    });
  });
});