import type { Options } from '@wdio/types';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';
import { fileURLToPath } from 'url';

// ESモジュールで__dirnameを取得
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// tauri-driverのパス
const tauriDriver = process.platform === 'win32' ? 'tauri-driver.exe' : 'tauri-driver';
let driverProcess: ChildProcess | null = null;

// msedgedriverのパスを探す
const msedgedriverPath = process.platform === 'win32'
  ? path.resolve(__dirname, '../msedgedriver.exe')
  : 'msedgedriver';

// Windows環境では.exe拡張子が必要
const appBinary = process.platform === 'win32'
  ? 'kukuri-tauri.exe'
  : 'kukuri-tauri';

// デバッグビルドかリリースビルドを使用
const buildType = process.env.E2E_BUILD_TYPE || 'release';
const appPath = path.resolve(__dirname, './src-tauri/target', buildType, appBinary);

// Tauriアプリケーションのパスを確認
if (!fs.existsSync(appPath)) {
  console.error(`ERROR: Application not found at ${appPath}`);
  console.error('Please run: pnpm tauri build --debug');
  process.exit(1);
}

// console.log(`Using application: ${appPath}`);

export const config: Options.Testrunner = {
  // テストファイル
  specs: ['./tests/e2e/**/*.spec.ts', './tests/e2e/specs/**/*.e2e.ts'],
  exclude: [],

  // 実行設定
  maxInstances: 1,

  // Capabilities - Tauriアプリケーション用
  capabilities: [{
    maxInstances: 1,
    // browserNameは指定しない（tauri-driverが自動設定）
    'tauri:options': {
      application: appPath,
    },
  }] as any,

  // WebDriver接続設定
  port: 4445,
  hostname: 'localhost',
  path: '/',

  // ログレベル
  logLevel: 'error', // エラーのみ表示

  // その他の設定
  bail: 0,
  waitforTimeout: 30000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  // テストフレームワーク
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  // tauri-driverを起動
  onPrepare: async function () {
    // console.log('Starting tauri-driver...');  // ログ抑制

    // 既存のドライバープロセスをクリーンアップ
    if (driverProcess) {
      driverProcess.kill();
      driverProcess = null;
    }

    return new Promise<void>((resolve, reject) => {
      // tauri-driverを起動（ポート4445で起動、msedgedriverはポート9515で起動）
      // console.log('Starting tauri-driver on port 4445...');
      // console.log('Using msedgedriver at:', msedgedriverPath);

      const args = [
        '--port', '4445',           // tauri-driverのポート
        '--native-port', '9515'      // msedgedriverのポート
      ];
      
      if (fs.existsSync(msedgedriverPath)) {
        args.push('--native-driver', msedgedriverPath);
      }

      driverProcess = spawn(tauriDriver, args, {
        stdio: ['ignore', 'pipe', 'pipe']  // pipeに戻すが、出力を確実に読み取る
      });

      // エラーハンドリング
      driverProcess.on('error', (error) => {
        console.error('Failed to start tauri-driver:', error);
        reject(error);
      });

      // プロセスが起動したら成功とみなす
      console.log('Starting tauri-driver process...');
      
      // エラー時のタイムアウト（念のため）
      const timeout = setTimeout(() => {
        if (driverProcess) {
          driverProcess.kill();
          driverProcess = null;
        }
        reject(new Error('tauri-driver process did not spawn within timeout'));
      }, 10000);

      // 出力を確実に読み取ってブロッキングを回避
      driverProcess.stdout?.on('data', (data) => {
        const output = data.toString();
        // 重要なメッセージのみ表示
        if (output.includes('Listening on') || output.includes('error')) {
          console.log('tauri-driver:', output.trim());
        }
      });

      driverProcess.stderr?.on('data', (data) => {
        const output = data.toString();
        // エラーは常に表示
        if (output.trim()) {
          console.error('tauri-driver stderr:', output.trim());
        }
      });

      // プロセス起動イベントを待つ
      driverProcess.on('spawn', () => {
        console.log('tauri-driver process spawned, waiting for initialization...');
        clearTimeout(timeout); // タイムアウトをクリア
        // tauri-driverが完全に起動するまで少し待つ
        setTimeout(() => {
          console.log('tauri-driver assumed ready');
          resolve();
        }, 3000);
      });
    });
  },

  // クリーンアップ
  onComplete: function () {
    // console.log('Cleaning up tauri-driver...');
    if (driverProcess) {
      driverProcess.kill();
      driverProcess = null;
    }
  },

  // セッション開始前
  beforeSession: function () {
    // 最小限のログのみ
    if (process.env.VERBOSE) {
      console.log('Starting new test session...');
      console.log('Application path:', appPath);
    }
  }
};
