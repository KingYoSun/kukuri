import path from 'node:path';

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

const env = (
  globalThis as typeof globalThis & {
    process?: {
      env?: Record<string, string | undefined>;
    };
  }
).process?.env;

const tauriDevHost = env?.KUKURI_TAURI_DEV_HOST ?? '127.0.0.1';
const rawTauriDevPort = Number.parseInt(env?.KUKURI_TAURI_DEV_PORT ?? '5173', 10);
const tauriDevPort =
  Number.isInteger(rawTauriDevPort) && rawTauriDevPort > 0 && rawTauriDevPort <= 65535
    ? rawTauriDevPort
    : 5173;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
    },
  },
  server: {
    host: tauriDevHost,
    port: tauriDevPort,
    strictPort: true,
  },
  test: {
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    // NOTE: include/exclude live on the projects below — `extends: true`
    // concatenates (not replaces) inherited arrays, so root-level patterns
    // would leak into every project.
    projects: [
      {
        extends: true,
        test: {
          name: 'unit',
          include: ['src/**/*.{test,spec}.{ts,tsx}', 'scripts/**/*.{test,spec}.mjs'],
          exclude: ['tests/playwright/**', 'src/shell/DesktopShellPage.*.test.tsx'],
          sequence: { groupOrder: 0 },
        },
      },
      {
        extends: true,
        test: {
          name: 'shell-integration',
          include: ['src/shell/DesktopShellPage.*.test.tsx'],
          // These suites mount the full App in jsdom and are timing-sensitive.
          // Before the WP-S1 split they all lived in one file, so they (a) ran
          // serially and (b) mostly ran after the parallel unit suites had
          // finished, i.e. on an otherwise idle CPU. Keep both properties:
          // one file at a time, scheduled after the unit project completes.
          maxWorkers: 1,
          sequence: { groupOrder: 1 },
        },
      },
    ],
  },
});
