import { join } from 'path';
import { spawn, ChildProcess } from 'child_process';
import type { Options } from '@wdio/types';

let tauriDriverProcess: ChildProcess | null = null;

export const config: Options.Testrunner = {
  // tauri-driverのホストとポート設定
  host: '127.0.0.1',
  port: 4445,
  
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json',
      transpileOnly: true
    }
  },
  
  specs: ['./tests/e2e/**/*.spec.ts'],
  exclude: [],
  maxInstances: 1,
  
  // Tauri専用のcapabilities設定
  capabilities: [{
    maxInstances: 1,
    'tauri:options': {
      application: join(process.cwd(), 'src-tauri/target/debug/kukuri-tauri.exe')
    }
  }],
  
  logLevel: 'info',
  bail: 0,
  waitforTimeout: 30000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000
  },
  
  // tauri-driverを起動する前にアプリケーションが存在することを確認
  onPrepare: async function () {
    const appPath = join(process.cwd(), 'src-tauri/target/debug/kukuri-tauri.exe');
    const { existsSync } = await import('fs');
    
    if (!existsSync(appPath)) {
      console.error(`Application not found at: ${appPath}`);
      console.log('Please run: pnpm tauri build --debug');
      process.exit(1);
    }
    
    console.log('Application found, ready for E2E testing');
  },
  
  // tauri-driverをbeforeSessionで起動（現在は手動起動のためコメントアウト）
  beforeSession: function () {
    console.log('Using manually started tauri-driver on port 4445...');
    // 手動起動したtauri-driverを使用するため、ここでは何もしない
    return Promise.resolve();
  },
  
  // テスト終了後にtauri-driverを停止
  afterSession: function () {
    // 手動起動の場合は何もしない
    console.log('Test session completed');
  }
};