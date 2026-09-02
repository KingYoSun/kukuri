import { expect, test } from 'vitest';

import { DESKTOP_LOCALE_STORAGE_KEY } from '@/i18n';
import { OS_NOTIFICATION_SETTINGS_STORAGE_KEY } from '@/lib/releaseReadiness';
import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';

import { applyPortableFrontendState, capturePortableFrontendState } from './deviceBackup';

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
