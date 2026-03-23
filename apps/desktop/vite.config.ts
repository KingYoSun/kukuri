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
    include: ['src/**/*.{test,spec}.{ts,tsx}', 'scripts/**/*.{test,spec}.mjs'],
    exclude: ['tests/playwright/**'],
  },
});
