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
    await this.waitForElement('#root');

    // 初期ロードが完了するまで少し待機
    await browser.pause(1000);
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
      const authStatus = await $('[data-testid="auth-status"]');
      const text = await authStatus.getText();
      return text.includes('Authenticated');
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

    // 鍵ペアを生成
    await this.clickButton('Generate Keypair');

    // 認証完了を待つ
    await browser.waitUntil(async () => await this.checkAuthStatus(), {
      timeout: 5000,
      timeoutMsg: 'Login failed',
    });
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
}
