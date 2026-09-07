import { beforeEach, expect, test, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({
  invoke,
  Channel: class { onmessage = () => {}; },
}));

import { check } from './appUpdater';

beforeEach(() => { invoke.mockReset(); });

test('backend owns target, verified bytes and install; frontend only passes the session', async () => {
  invoke.mockResolvedValueOnce({ id: 7, version: '0.1.9' }).mockResolvedValue(undefined);
  const update = await check();
  expect(invoke).toHaveBeenCalledWith('check_app_update');
  await update!.download();
  expect(invoke).toHaveBeenLastCalledWith('download_app_update', { id: 7, onEvent: expect.any(Object) });
  await update!.install();
  expect(invoke).toHaveBeenLastCalledWith('install_app_update', { id: 7 });
  expect(invoke.mock.calls.some(([name]) => String(name).startsWith('plugin:updater'))).toBe(false);
});

test('no update yields no install handle', async () => {
  invoke.mockResolvedValue(null);
  expect(await check()).toBeNull();
  expect(invoke).toHaveBeenCalledTimes(1);
});

test.each(['deb_update_auth_cancelled', 'deb_update_auth_unavailable', 'deb_update_install_failed'])(
  'a rejected install (%s) is returned without retry or alternate authentication', async (error) => {
    invoke.mockResolvedValueOnce({ id: 8, version: '0.1.9' }).mockRejectedValue(error);
    const update = await check();
    await expect(update!.install()).rejects.toBe(error);
    expect(invoke).toHaveBeenCalledTimes(2);
  },
);
