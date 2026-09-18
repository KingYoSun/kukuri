import path from 'node:path';

import { defineConfig, devices } from '@playwright/test';

/**
 * 告知素材の撮影用 Playwright 設定 (#1038)。
 *
 * 既存の playwright.config.ts とは次を分離する。
 * - port: 4176 (既存) と 4177 (promo)
 * - build 出力: dist (既存) と dist-promo (promo)
 * - test 選択: tests/playwright (既存) と tests/promo (promo)
 * - artifact: test-results (既存) と promo-artifacts (promo)
 *
 * 視覚回帰の baseline は扱わない。snapshot 比較を行わないため、
 * 既存の __screenshots__ には触れない。
 */

const port = 4177;
const host = '127.0.0.1';
const baseURL = `http://${host}:${port}`;
const distDir = 'dist-promo';

export default defineConfig({
  testDir: './tests/promo',
  // 撮影は画面の状態に依存するため、並行実行しない。
  fullyParallel: false,
  workers: 1,
  // 失敗した撮影を retry で上書きせず、失敗として報告する。
  retries: 0,
  reporter: 'list',
  // Playwright 自身の artifact も既存の test-results から分ける。
  outputDir: path.resolve(import.meta.dirname, '../../promo-artifacts/playwright-output'),
  // 撮影後に原素材全体の索引 (captures/index.json) を書く。
  globalTeardown: './tests/promo/writeCaptureIndex.ts',
  timeout: 120_000,
  use: {
    baseURL,
    locale: 'ja-JP',
    timezoneId: 'Asia/Tokyo',
    trace: 'off',
    // 既存 test と同じく、毎回使い捨ての context で動く。
    // userDataDir を指定しないため、開発機の既存 profile を上書きしない。
    ...devices['Desktop Chrome'],
  },
  webServer: {
    command:
      `node ./node_modules/vite/bin/vite.js build --outDir ${distDir} && ` +
      `node ./node_modules/vite/bin/vite.js preview --outDir ${distDir} --host ${host} --port ${port} --strictPort`,
    cwd: path.resolve(import.meta.dirname),
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    timeout: 300_000,
    env: {
      ...process.env,
      VITE_KUKURI_DESKTOP_MOCK: '1',
    },
  },
});
