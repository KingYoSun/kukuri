import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import { runtimeApi } from './api';

describe('runtimeApi invoke errors', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    delete window.__KUKURI_DESKTOP__;
  });

  it('preserves backend command errors for createPost', async () => {
    invokeMock.mockRejectedValueOnce('reply target missing');

    await expect(
      runtimeApi.createPost('kukuri:topic:test', 'reply body', 'root-id', [])
    ).rejects.toThrow('reply target missing');
  });

  it('normalizes tauri bridge attachment failures', async () => {
    invokeMock.mockRejectedValueOnce(new Error('__TAURI_INTERNALS__ was not found'));

    await expect(runtimeApi.getSyncStatus()).rejects.toThrow('Desktop backend is not attached.');
  });
});
