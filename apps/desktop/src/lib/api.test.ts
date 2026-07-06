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

  // WP-C3: バックエンドは { code, message } の構造化封筒で reject する。
  // message は従来の平文文言と同一に保たれ、code が機械判定用に付く。
  it('exposes a machine-readable code on structured backend errors', async () => {
    invokeMock.mockRejectedValueOnce({ code: 'command_failed', message: 'reply target missing' });

    const error = await runtimeApi.getSyncStatus().catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(Error);
    expect((error as Error).message).toBe('reply target missing');
    expect((error as Error & { code?: string }).code).toBe('command_failed');
  });

  it('exposes bridge_unavailable code on tauri bridge failures', async () => {
    invokeMock.mockRejectedValueOnce(new Error('__TAURI_INTERNALS__ was not found'));

    const error = await runtimeApi.getSyncStatus().catch((caught: unknown) => caught);
    expect((error as Error).message).toBe('Desktop backend is not attached.');
    expect((error as Error & { code?: string }).code).toBe('bridge_unavailable');
  });

  it('marks plain string rejections as command failures with the message preserved', async () => {
    invokeMock.mockRejectedValueOnce('reply target missing');

    const error = await runtimeApi.getSyncStatus().catch((caught: unknown) => caught);
    expect((error as Error).message).toBe('reply target missing');
    expect((error as Error & { code?: string }).code).toBe('command_failed');
  });
});
