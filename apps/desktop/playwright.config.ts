import path from 'node:path';

import { defineConfig, devices } from '@playwright/test';

const port = 4176;
const host = '127.0.0.1';
const baseURL = `http://${host}:${port}`;

export default defineConfig({
  testDir: './tests/playwright',
  fullyParallel: true,
  reporter: 'list',
  use: {
    baseURL,
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
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
