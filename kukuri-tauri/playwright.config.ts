import { defineConfig, devices } from '@playwright/test';

const host = process.env.PLAYWRIGHT_HOST ?? '127.0.0.1';
const port = Number(process.env.PLAYWRIGHT_PORT ?? '1420');
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? `http://${host}:${port}`;
const adminHost = process.env.PLAYWRIGHT_ADMIN_HOST ?? host;
const adminPort = Number(process.env.PLAYWRIGHT_ADMIN_PORT ?? '4173');
const adminBaseURL =
  process.env.PLAYWRIGHT_ADMIN_BASE_URL ?? `http://${adminHost}:${adminPort}`;

export default defineConfig({
  testDir: './e2e',
  testMatch: ['smoke/**/*.spec.ts'],
  fullyParallel: false,
  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : [['list']],
  use: {
    baseURL,
    trace: 'on-first-retry',
    screenshot: 'on',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: [
    {
      command: `pnpm dev --host ${host} --port ${port} --strictPort`,
      url: baseURL,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
    {
      command:
        `pnpm --dir ../kukuri-community-node/apps/admin-console dev ` +
        `--host ${adminHost} --port ${adminPort} --strictPort`,
      url: adminBaseURL,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
  ],
});
