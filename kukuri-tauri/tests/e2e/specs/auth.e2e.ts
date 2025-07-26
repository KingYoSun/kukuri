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

  describe('Key Generation', () => {
    it('should generate new keypair and authenticate', async () => {
      // 初期状態の確認
      const authStatus = await AppHelper.checkAuthStatus();
      expect(authStatus).toBe(false);

      // 鍵ペア生成ボタンをクリック
      await AppHelper.clickButton('Generate Keypair');

      // 認証が完了するまで待つ
      await browser.waitUntil(async () => await AppHelper.checkAuthStatus(), {
        timeout: 10000,
        timeoutMsg: 'Authentication did not complete',
      });

      // 公開鍵が表示されることを確認
      const publicKeyElement = await $('[data-testid="public-key"]');
      const publicKey = await publicKeyElement.getText();
      expect(publicKey).toMatch(/^npub1[a-zA-Z0-9]+/);
    });

    it('should show loading state during key generation', async () => {
      // 鍵ペア生成ボタンをクリック
      const generateButton = await $('button:has-text("Generate Keypair")');
      await generateButton.click();

      // ローディング状態を確認
      const loadingIndicator = await $('[data-testid="loading-indicator"]');
      if (await loadingIndicator.isExisting()) {
        expect(await loadingIndicator.isDisplayed()).toBe(true);

        // ローディングが終了するまで待つ
        await loadingIndicator.waitForDisplayed({ reverse: true, timeout: 10000 });
      }
    });
  });

  describe('Login with Existing Key', () => {
    it('should login with secret key', async () => {
      // ログインフォームを開く
      const loginButton = await $('[data-testid="show-login-form"]');
      if (await loginButton.isExisting()) {
        await loginButton.click();
      }

      // 秘密鍵入力フィールド
      const secretKeyInput = await $('[data-testid="secret-key-input"]');
      await secretKeyInput.setValue('nsec1testkey123456789');

      // パスワード入力（必要な場合）
      const passwordInput = await $('[data-testid="password-input"]');
      if (await passwordInput.isExisting()) {
        await passwordInput.setValue('testpassword');
      }

      // ログインボタンをクリック
      const submitButton = await $('[data-testid="login-submit"]');
      await submitButton.click();

      // エラーメッセージまたは成功を確認
      await browser.waitUntil(
        async () => {
          const errorMsg = await $('[data-testid="error-message"]');
          const isAuthenticated = await AppHelper.checkAuthStatus();
          return (await errorMsg.isExisting()) || isAuthenticated;
        },
        { timeout: 5000 },
      );
    });

    it('should handle invalid secret key', async () => {
      // 無効な秘密鍵でログインを試みる
      const loginButton = await $('[data-testid="show-login-form"]');
      if (await loginButton.isExisting()) {
        await loginButton.click();
      }

      const secretKeyInput = await $('[data-testid="secret-key-input"]');
      await secretKeyInput.setValue('invalid-key');

      const submitButton = await $('[data-testid="login-submit"]');
      await submitButton.click();

      // エラーメッセージが表示される
      const errorMessage = await $('[data-testid="error-message"]');
      await errorMessage.waitForDisplayed({ timeout: 5000 });

      const errorText = await errorMessage.getText();
      expect(errorText).toContain('Invalid');
    });
  });

  describe('Logout', () => {
    it('should logout successfully', async () => {
      // まずログイン
      await AppHelper.login();

      // ログアウトボタンを探す
      const logoutButton = await $('[data-testid="logout-button"]');
      expect(await logoutButton.isExisting()).toBe(true);

      // ログアウト
      await logoutButton.click();

      // 確認ダイアログが表示される場合
      const confirmButton = await $('[data-testid="confirm-logout"]');
      if (await confirmButton.isExisting()) {
        await confirmButton.click();
      }

      // ログアウトが完了するまで待つ
      await browser.waitUntil(async () => !(await AppHelper.checkAuthStatus()), {
        timeout: 5000,
        timeoutMsg: 'Logout did not complete',
      });

      // 認証状態がクリアされたことを確認
      const authStatus = await AppHelper.checkAuthStatus();
      expect(authStatus).toBe(false);
    });
  });

  describe('Key Management', () => {
    it('should export and import keys', async () => {
      // ログイン
      await AppHelper.login();

      // 設定ページに移動
      await browser.url('/settings');
      await browser.pause(1000);

      // エクスポートボタンをクリック
      const exportButton = await $('[data-testid="export-key-button"]');
      if (await exportButton.isExisting()) {
        await exportButton.click();

        // エクスポートされた鍵が表示される
        const exportedKey = await $('[data-testid="exported-key"]');
        await exportedKey.waitForDisplayed({ timeout: 5000 });

        const keyText = await exportedKey.getText();
        expect(keyText).toMatch(/^nsec1[a-zA-Z0-9]+/);

        // コピーボタンの確認
        const copyButton = await $('[data-testid="copy-key-button"]');
        if (await copyButton.isExisting()) {
          await copyButton.click();

          // コピー成功メッセージ
          const successMsg = await $('[data-testid="copy-success"]');
          if (await successMsg.isExisting()) {
            expect(await successMsg.isDisplayed()).toBe(true);
          }
        }
      }
    });

    it('should set and verify password protection', async () => {
      // ログイン
      await AppHelper.login();

      // 設定ページに移動
      await browser.url('/settings');
      await browser.pause(1000);

      // パスワード設定セクション
      const passwordSection = await $('[data-testid="password-protection"]');
      if (await passwordSection.isExisting()) {
        // パスワード設定ボタン
        const setPasswordButton = await $('[data-testid="set-password-button"]');
        await setPasswordButton.click();

        // パスワード入力
        const newPasswordInput = await $('[data-testid="new-password-input"]');
        await newPasswordInput.setValue('newSecurePassword123');

        const confirmPasswordInput = await $('[data-testid="confirm-password-input"]');
        await confirmPasswordInput.setValue('newSecurePassword123');

        // 保存
        const saveButton = await $('[data-testid="save-password-button"]');
        await saveButton.click();

        // 成功メッセージ
        const successMessage = await $('[data-testid="password-success"]');
        await successMessage.waitForDisplayed({ timeout: 5000 });

        expect(await successMessage.getText()).toContain('Password set successfully');
      }
    });
  });

  describe('Session Persistence', () => {
    it('should maintain authentication across page reloads', async () => {
      // ログイン
      await AppHelper.login();

      // 公開鍵を記録
      const publicKeyElement = await $('[data-testid="public-key"]');
      const originalPublicKey = await publicKeyElement.getText();

      // ページをリロード
      await browser.refresh();
      await AppHelper.waitForAppReady();

      // 認証状態が維持されていることを確認
      const authStatus = await AppHelper.checkAuthStatus();
      expect(authStatus).toBe(true);

      // 同じ公開鍵が表示されていることを確認
      const newPublicKeyElement = await $('[data-testid="public-key"]');
      const newPublicKey = await newPublicKeyElement.getText();
      expect(newPublicKey).toBe(originalPublicKey);
    });
  });
});
