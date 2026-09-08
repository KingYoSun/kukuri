import { expect, test } from 'vitest';

import i18n, { DESKTOP_LOCALE_STORAGE_KEY, normalizeSupportedLocale } from './index';
import { changeDesktopLocale } from './changeLocale';

test('normalizeSupportedLocale maps supported and fallback locales', () => {
  expect(normalizeSupportedLocale('en-US')).toBe('en');
  expect(normalizeSupportedLocale('ja-JP')).toBe('ja');
  expect(normalizeSupportedLocale('zh-CN')).toBe('zh-CN');
  expect(normalizeSupportedLocale('zh')).toBe('zh-CN');
  expect(normalizeSupportedLocale('zh-Hans')).toBe('zh-CN');
  expect(normalizeSupportedLocale('fr-FR')).toBe('en');
  expect(normalizeSupportedLocale(null)).toBe('en');
});

test('an explicit language choice persists the selected locale in localStorage', () => {
  expect(changeDesktopLocale('zh-CN')).toBe(true);

  expect(window.localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('zh-CN');
  expect(i18n.resolvedLanguage).toBe('zh-CN');
});
