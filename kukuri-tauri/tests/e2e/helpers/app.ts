export class AppHelper {
  /**
   * アプリのメインウィンドウを取得
   */
  static async getMainWindow() {
    const windows = await browser.getWindowHandles();
    if (windows.length === 0) {
      throw new Error('No windows found');
    }
    await browser.switchToWindow(windows[0]);
    return windows[0];
  }

  /**
   * 要素が表示されるまで待機
   */
  static async waitForElement(selector: string, timeout = 10000) {
    const element = await $(selector);
    await element.waitForDisplayed({ timeout });
    return element;
  }

  /**
   * テキストを含む要素を検索
   */
  static async findByText(text: string, tag = '*') {
    return await $(`//${tag}[contains(text(), "${text}")]`);
  }

  /**
   * ボタンをクリック
   */
  static async clickButton(text: string) {
    const button = await this.findByText(text, 'button');
    await button.waitForClickable();
    await button.click();
  }

  /**
   * 入力フィールドに値を設定
   */
  static async setInputValue(selector: string, value: string) {
    const input = await $(selector);
    await input.waitForDisplayed();
    await input.clearValue();
    await input.setValue(value);
  }

  /**
   * アプリが起動するまで待機
   */
  static async waitForAppReady() {
    // アプリのルート要素が表示されるまで待機
    await this.waitForElement('#root', 15000);

    // Reactアプリケーションが完全にレンダリングされるまで待機
    await browser.waitUntil(
      async () => {
        // #root要素内に子要素が存在することを確認
        const rootChildren = await browser.execute(() => {
          const root = document.getElementById('root');
          return root ? root.children.length > 0 : false;
        });
        
        // bodyタグが表示されていることを確認
        const bodyVisible = await browser.execute(() => {
          const body = document.querySelector('body');
          return body ? window.getComputedStyle(body).display !== 'none' : false;
        });

        return rootChildren && bodyVisible;
      },
      {
        timeout: 20000,
        timeoutMsg: 'App failed to load within 20 seconds',
        interval: 500
      }
    );

    // アニメーションやトランジションが完了するための追加の待機
    await browser.pause(500);
  }

  /**
   * スクリーンショットを撮影
   */
  static async takeScreenshot(name: string) {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    await browser.saveScreenshot(`./tests/e2e/screenshots/${name}-${timestamp}.png`);
  }

  /**
   * 認証状態を確認
   */
  static async checkAuthStatus(): Promise<boolean> {
    try {
      // URL ベースでの確認
      const url = await browser.getUrl();
      
      // 認証ページにいる場合は未認証
      if (url.includes('/welcome') || url.includes('/login')) {
        return false;
      }
      
      // data-testid="auth-status" 要素があれば確認
      const authStatus = await $('[data-testid="auth-status"]');
      if (await authStatus.isExisting()) {
        const text = await authStatus.getText();
        return text.includes('Authenticated');
      }
      
      // プロテクトされたページにアクセスできる場合は認証済み
      if (url.includes('/topics') || url.includes('/settings') || url === '/' || url.endsWith('/')) {
        return true;
      }
      
      return false;
    } catch {
      return false;
    }
  }

  /**
   * ログイン処理
   */
  static async login() {
    // 既にログイン済みの場合はスキップ
    if (await this.checkAuthStatus()) {
      return;
    }

    // Welcomeページにいることを確認
    const url = await browser.getUrl();
    if (!url.includes('/welcome')) {
      const baseUrl = url.split('#')[0].split('?')[0];
      await browser.url(baseUrl + '#/welcome');
      await this.waitForElement('body');
    }

    // 新規アカウント作成ボタンをクリック
    try {
      await this.clickButton('新規アカウント作成');
    } catch {
      // フォールバック: 英語版のボタンテキスト
      await this.clickButton('Generate Keypair');
    }

    // プロファイル設定ページへのリダイレクトを待つ
    await browser.waitUntil(
      async () => {
        const currentUrl = await browser.getUrl();
        return currentUrl.includes('/profile-setup') || !currentUrl.includes('/welcome');
      },
      {
        timeout: 10000,
        timeoutMsg: 'Login failed',
      }
    );
  }

  /**
   * トピック一覧を取得
   */
  static async getTopicList(): Promise<string[]> {
    const topicsList = await $('[data-testid="topics-list"]');
    const topics = await topicsList.$$('[data-testid^="topic-"]');

    const topicNames: string[] = [];
    for (const topic of topics) {
      const nameElement = await topic.$('h3');
      if (nameElement) {
        const name = await nameElement.getText();
        topicNames.push(name);
      }
    }

    return topicNames;
  }

  /**
   * 投稿一覧を取得
   */
  static async getPostList(): Promise<Array<{ id: string; content: string }>> {
    const postsList = await $('[data-testid="posts-list"]');
    const posts = await postsList.$$('[data-testid^="post-"]');

    const postData: Array<{ id: string; content: string }> = [];
    for (const post of posts) {
      const testId = await post.getAttribute('data-testid');
      const id = testId.replace('post-', '');
      const contentElement = await post.$('p');
      if (contentElement) {
        const content = await contentElement.getText();
        postData.push({ id, content });
      }
    }

    return postData;
  }

  /**
   * リレー接続状態を取得
   */
  static async getRelayStatus(): Promise<Record<string, string>> {
    const relayList = await $('[data-testid="relay-list"]');
    const relays = await relayList.$$('[data-testid^="relay-"]');

    const status: Record<string, string> = {};
    for (const relay of relays) {
      const urlElement = await relay.$('span:first-child');
      const statusElement = await relay.$('[data-testid^="status-"]');

      if (urlElement && statusElement) {
        const url = await urlElement.getText();
        const relayStatus = await statusElement.getText();
        status[url] = relayStatus;
      }
    }

    return status;
  }

  /**
   * 認証を実行して認証済み状態にする
   */
  static async ensureAuthenticated() {
    // 既に認証済みならスキップ
    if (await this.checkAuthStatus()) {
      return;
    }

    // ログイン処理を実行
    await this.login();

    // プロファイル設定ページにいる場合はスキップ
    const url = await browser.getUrl();
    if (url.includes('/profile-setup')) {
      try {
        const skipBtn = await this.findByText('スキップ', 'button');
        if (await skipBtn.isExisting()) {
          await skipBtn.click();
          await browser.pause(1000);
        }
      } catch {
        // スキップボタンがない場合は保存ボタンを試す
        try {
          const saveBtn = await this.findByText('保存', 'button');
          if (await saveBtn.isExisting()) {
            await saveBtn.click();
            await browser.pause(1000);
          }
        } catch {
          // ボタンがない場合は直接ホームページへ
          const currentUrl = await browser.getUrl();
          const baseUrl = currentUrl.split('#')[0].split('?')[0];
          await browser.url(baseUrl + '#/');
        }
      }
    }
  }

  /**
   * ナビゲーションメニューからページに移動
   */
  static async navigateToPage(pageName: 'home' | 'topics' | 'settings' | 'profile') {
    const navLinks: Record<string, string> = {
      home: '/',
      topics: '/topics',
      settings: '/settings',
      profile: '/profile',
    };

    const currentUrl = await browser.getUrl();
    const baseUrl = currentUrl.split('#')[0].split('?')[0];
    await browser.url(baseUrl + '#' + navLinks[pageName]);
    await browser.waitUntil(
      async () => {
        const url = await browser.getUrl();
        return url.includes(navLinks[pageName]);
      },
      {
        timeout: 5000,
        timeoutMsg: `Failed to navigate to ${pageName} page`,
      }
    );
  }
}
