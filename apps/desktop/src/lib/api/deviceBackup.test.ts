import { beforeEach, expect, test, vi } from 'vitest';

import { DESKTOP_LOCALE_STORAGE_KEY } from '@/i18n';
import { OS_NOTIFICATION_SETTINGS_STORAGE_KEY } from '@/lib/releaseReadiness';
import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import {
  applyPendingDeviceRestoreFrontendState,
  applyPortableFrontendState,
  capturePortableFrontendState,
} from './deviceBackup';

beforeEach(() => {
  invokeMock.mockReset();
  window.localStorage.clear();
  delete window.__KUKURI_DESKTOP__;
});

test('captures only the reviewed portable frontend storage allowlist', () => {
  window.localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, 'ja');
  window.localStorage.setItem(DESKTOP_THEME_STORAGE_KEY, 'dark');
  window.localStorage.setItem(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, '{"enabled":true}');
  window.localStorage.setItem('unreviewed-secret', 'must-not-leave-device');

  expect(capturePortableFrontendState()).toEqual({
    [DESKTOP_LOCALE_STORAGE_KEY]: 'ja',
    [DESKTOP_THEME_STORAGE_KEY]: 'dark',
  });
});

test('restores only allowlisted keys even if an archive contains unknown entries', () => {
  applyPortableFrontendState({
    [DESKTOP_LOCALE_STORAGE_KEY]: 'zh-CN',
    'unreviewed-secret': 'must-not-be-written',
  });

  expect(window.localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('zh-CN');
  expect(window.localStorage.getItem('unreviewed-secret')).toBeNull();
});

test('applies restored frontend state only from an activated backend marker and acknowledges it', async () => {
  window.localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, 'en');
  invokeMock
    .mockResolvedValueOnce({
      [DESKTOP_LOCALE_STORAGE_KEY]: 'ja',
      [DESKTOP_THEME_STORAGE_KEY]: 'light',
      'unreviewed-secret': 'must-not-be-written',
    })
    .mockResolvedValueOnce(undefined);

  await expect(applyPendingDeviceRestoreFrontendState()).resolves.toBe(true);

  expect(window.localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('ja');
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBe('light');
  expect(window.localStorage.getItem('unreviewed-secret')).toBeNull();
  expect(invokeMock).toHaveBeenNthCalledWith(
    1,
    'get_pending_device_restore_frontend_state',
    undefined
  );
  expect(invokeMock).toHaveBeenNthCalledWith(
    2,
    'acknowledge_pending_device_restore_frontend_state',
    undefined
  );
});

test('restores the previous frontend state when marker acknowledgement fails and can retry', async () => {
  window.localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, 'en');
  const restored = {
    [DESKTOP_LOCALE_STORAGE_KEY]: 'ja',
    [DESKTOP_THEME_STORAGE_KEY]: 'light',
  };
  invokeMock
    .mockResolvedValueOnce(restored)
    .mockRejectedValueOnce(new Error('ack failed'))
    .mockResolvedValueOnce(restored)
    .mockResolvedValueOnce(undefined);

  await expect(applyPendingDeviceRestoreFrontendState()).rejects.toThrow('ack failed');
  expect(window.localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('en');
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBeNull();

  await expect(applyPendingDeviceRestoreFrontendState()).resolves.toBe(true);
  expect(window.localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('ja');
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBe('light');
});

test('rolls back partial localStorage writes when applying restored state fails', async () => {
  const values = new Map<string, string>([
    [DESKTOP_LOCALE_STORAGE_KEY, 'en'],
  ]);
  let failThemeWrite = true;
  const storage: Storage = {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => {
      values.delete(key);
    },
    setItem: (key, value) => {
      if (key === DESKTOP_THEME_STORAGE_KEY && failThemeWrite) {
        failThemeWrite = false;
        throw new Error('storage full');
      }
      values.set(key, value);
    },
  };
  invokeMock.mockResolvedValueOnce({
    [DESKTOP_LOCALE_STORAGE_KEY]: 'ja',
    [DESKTOP_THEME_STORAGE_KEY]: 'light',
  });

  await expect(applyPendingDeviceRestoreFrontendState(storage)).rejects.toThrow('storage full');
  expect(storage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('en');
  expect(storage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBeNull();
  expect(invokeMock).toHaveBeenCalledTimes(1);
});
