/// <reference types="vitest" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/tests/setup.ts',
    css: false, // CSSの処理を無効化
    testTimeout: 15000, // SearchErrorState artefact や offline sync のタイマー待機に余裕を持たせる
    exclude: ['node_modules', 'dist', '.idea', '.git', '.cache', 'tests/e2e/**', 'e2e/**'],
    typecheck: {
      tsconfig: './tsconfig.test.json',
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@tauri-apps/plugin-dialog': path.resolve(__dirname, './src/tests/mocks/tauri-dialog.ts'),
      '@tauri-apps/plugin-fs': path.resolve(__dirname, './src/tests/mocks/tauri-fs.ts'),
    },
  },
});
