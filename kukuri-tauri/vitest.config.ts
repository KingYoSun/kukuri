/// <reference types="vitest" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    css: false, // CSSの処理を無効化
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})