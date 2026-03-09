import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const allowedHosts = (env.VITE_ALLOWED_HOSTS ?? '')
    .split(',')
    .map((host) => host.trim())
    .filter((host) => host.length > 0);

  return {
    base: '/admin/',
    plugins: [react()],
    server: {
      host: '0.0.0.0',
      port: 5173,
      allowedHosts
    },
    preview: {
      host: '0.0.0.0',
      port: 5173
    },
    test: {
      environment: 'jsdom',
      globals: true,
      setupFiles: './src/test/setup.ts',
      css: true,
      include: ['src/**/*.test.{ts,tsx}']
    }
  }
});
