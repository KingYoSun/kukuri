/**
 * E2Eテストデバッグヘルパー
 */

// デバッグログを制御する環境変数
const DEBUG = process.env.E2E_DEBUG === 'true' || process.env.VERBOSE === 'true';

/**
 * デバッグログを出力
 */
function debugLog(...args: any[]) {
  if (DEBUG) {
    console.log(...args);
  }
}

/**
 * 現在のブラウザ状態をログに出力
 */
export async function logBrowserState() {
  // デバッグモードの場合のみ実行
  if (!DEBUG) return;

  try {
    // ウィンドウハンドルを取得
    const handles = await browser.getWindowHandles();
    debugLog('Window handles:', handles);

    if (handles.length > 0) {
      // 最初のウィンドウに切り替え
      await browser.switchToWindow(handles[0]);

      // タイトルを取得
      const title = await browser.getTitle();
      debugLog('Window title:', title);

      // URLを取得
      const url = await browser.getUrl();
      debugLog('Current URL:', url);

      // HTML全体を取得（最初の100文字のみ）
      const html = await browser.execute(() => document.documentElement.outerHTML);
      debugLog('HTML preview:', html.substring(0, 100) + '...');

      // body要素の存在確認
      const bodyExists = await browser.execute(() => !!document.body);
      debugLog('Body exists:', bodyExists);

      // #root要素の存在確認
      const rootExists = await browser.execute(() => !!document.getElementById('root'));
      debugLog('#root exists:', rootExists);

      // 全div要素の数
      const divCount = await browser.execute(() => document.querySelectorAll('div').length);
      debugLog('Div count:', divCount);
    } else {
      debugLog('No window handles found!');
    }
  } catch (error) {
    if (DEBUG) {
      console.error('Error getting browser state:', error);
    }
  }
}

/**
 * Tauriアプリケーションの起動を待つ
 */
export async function waitForTauriApp(timeout = 10000) {
  debugLog('Waiting for Tauri app to start...');

  await browser.waitUntil(
    async () => {
      try {
        const handles = await browser.getWindowHandles();
        if (handles.length === 0) {
          debugLog('No window handles yet...');
          return false;
        }

        // 最初のウィンドウに切り替え
        await browser.switchToWindow(handles[0]);

        // タイトルがセットされているか確認
        const title = await browser.getTitle();
        debugLog('Current title:', title);

        // bodyが存在するか確認
        const bodyExists = await browser.execute(() => !!document.body);

        return bodyExists;
      } catch (error) {
        debugLog('Still waiting for app...', error.message);
        return false;
      }
    },
    {
      timeout,
      timeoutMsg: 'Tauri app failed to start',
      interval: 2000
    }
  );

  debugLog('Tauri app started successfully');

  // アプリケーションが完全に初期化されるまで追加の待機
  await browser.pause(2000);
}
