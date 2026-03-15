// @vitest-environment node

import { describe, expect, it } from 'vitest';

import tauriConfig from '../src-tauri/tauri.conf.json';
import viteConfig from '../vite.config';

describe('desktop dev config', () => {
  it('pins the tauri dev server to a fixed IPv4 loopback URL', () => {
    expect(tauriConfig.build.devUrl).toBe('http://127.0.0.1:5173');
    expect(tauriConfig.build.beforeDevCommand).toContain('--host 127.0.0.1');
    expect(tauriConfig.build.beforeDevCommand).toContain('--port 5173');
    expect(tauriConfig.build.beforeDevCommand).toContain('--strictPort');
  });

  it('pins the vite dev server to the same host and port', () => {
    expect(viteConfig.server).toMatchObject({
      host: '127.0.0.1',
      port: 5173,
      strictPort: true,
    });
  });
});
