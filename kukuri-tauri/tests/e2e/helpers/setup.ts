import { AppHelper } from './app';

/**
 * E2Eテストのセットアップ
 */
export async function setupE2ETest() {
  // ブラウザのビューポートサイズを設定
  await browser.setWindowSize(1280, 800);

  // アプリの起動を待つ
  await AppHelper.waitForAppReady();

  // 初期状態のスクリーンショットを撮影（デバッグ用）
  if (process.env.E2E_SCREENSHOT) {
    await AppHelper.takeScreenshot('initial-state');
  }
}

/**
 * 各テストの前処理
 */
export async function beforeEachTest() {
  // アプリケーションの状態をリセット
  await browser.execute(() => {
    // localStorageをクリア
    window.localStorage.clear();
    // sessionStorageもクリア
    window.sessionStorage.clear();
  });

  // セキュアストレージのアカウントデータをクリア（別のexecuteで実行）
  try {
    // Tauri APIを使ってセキュアストレージをクリア
    const result = await browser.execute(() => {
      return new Promise((resolve) => {
        // @ts-ignore - Tauri APIは実行時に利用可能
        if (window.__TAURI__ && window.__TAURI__.core) {
          const { invoke } = window.__TAURI__.core;
          invoke('clear_all_accounts_for_test')
            .then(() => {
              console.log('Secure storage cleared for E2E test');
              resolve({ success: true });
            })
            .catch((error: any) => {
              console.warn('Failed to clear secure storage:', error);
              resolve({ success: false, error: error.toString() });
            });
        } else {
          console.log('Tauri API not available, skipping secure storage clear');
          resolve({ success: false, error: 'Tauri API not available' });
        }
      });
    });
    console.log('Clear secure storage result:', result);
  } catch (error) {
    console.warn('Failed to execute secure storage clear:', error);
  }

  // ページをリロード
  await browser.refresh();

  // アプリの再起動を待つ
  await AppHelper.waitForAppReady();
}

/**
 * 各テストの後処理
 */
export async function afterEachTest(testName: string) {
  // エラーが発生した場合はスクリーンショットを保存
  if (browser.capabilities && testName) {
    const testStatus = await browser.execute(() => {
      // テストのステータスを取得する実装
      return 'passed'; // 実際にはテストフレームワークから取得
    });

    if (testStatus !== 'passed') {
      await AppHelper.takeScreenshot(`error-${testName}`);
    }
  }
}

/**
 * テストデータのクリーンアップ
 */
export async function cleanupTestData() {
  // テスト用のデータを削除
  // 実際のアプリでは、テスト用のクリーンアップコマンドを実装
  await browser.execute(() => {
    // IndexedDBのクリア
    if ('indexedDB' in window) {
      indexedDB.databases().then((databases) => {
        databases.forEach((db) => {
          if (db.name) {
            indexedDB.deleteDatabase(db.name);
          }
        });
      });
    }
  });
}
