import path from 'node:path';

import { defineConfig, devices } from '@playwright/test';

const port = 4176;
const host = '127.0.0.1';
const baseURL = `http://${host}:${port}`;

// 視覚回帰 spec のファイル名。機能 e2e project からは testIgnore で除外し、
// 視覚 project からは testMatch で拾う。両者を 1 ファイル名で一元管理する。
const VISUAL_SPEC = '**/visual.spec.ts';

export default defineConfig({
  testDir: './tests/playwright',
  fullyParallel: true,
  reporter: 'list',
  // baseline は Linux CI 生成に一本化する（@font-face 非同梱でシステムフォント依存のため
  // Windows 開発機との pixel 一致は構造的に不可能）。CI 以外では比較を skip し、視覚 spec は
  // 到達操作の smoke として流れる。cargo xtask desktop-ui-check は従来どおり green のまま。
  ignoreSnapshots: !process.env.CI,
  // プラットフォーム suffix を付けない（Linux 固定運用で -win32 混入事故を防ぐ）。
  snapshotPathTemplate: '{testDir}/__screenshots__/{testFileName}/{arg}{ext}',
  use: {
    baseURL,
    locale: 'en-US',
    timezoneId: 'UTC',
    trace: 'on-first-retry',
  },
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.01,
      animations: 'disabled',
      caret: 'hide',
    },
  },
  projects: [
    {
      // 機能 e2e（既存 10 E2E）。視覚 spec は除外し、機能レーンを不変に保つ。
      name: 'chromium',
      testIgnore: VISUAL_SPEC,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
    {
      // 視覚回帰（visual.spec.ts のみ）。
      name: 'visual',
      testMatch: VISUAL_SPEC,
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
  webServer: {
    command: `node ./node_modules/vite/bin/vite.js build && node ./node_modules/vite/bin/vite.js preview --host ${host} --port ${port} --strictPort`,
    cwd: path.resolve(import.meta.dirname),
    url: baseURL,
    reuseExistingServer: !process.env.CI,
    env: {
      ...process.env,
      VITE_KUKURI_DESKTOP_MOCK: '1',
    },
  },
});
