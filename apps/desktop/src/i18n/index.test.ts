import { expect, test } from 'vitest';

import i18n, { DESKTOP_LOCALE_STORAGE_KEY, normalizeSupportedLocale } from './index';

test('normalizeSupportedLocale maps supported and fallback locales', () => {
  expect(normalizeSupportedLocale('en-US')).toBe('en');
  expect(normalizeSupportedLocale('ja-JP')).toBe('ja');
  expect(normalizeSupportedLocale('zh-CN')).toBe('zh-CN');
  expect(normalizeSupportedLocale('zh')).toBe('zh-CN');
  expect(normalizeSupportedLocale('zh-Hans')).toBe('zh-CN');
  expect(normalizeSupportedLocale('fr-FR')).toBe('en');
  expect(normalizeSupportedLocale(null)).toBe('en');
});

test('changeLanguage persists the selected locale in localStorage', async () => {
  await i18n.changeLanguage('zh-CN');

  expect(window.localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('zh-CN');
  expect(i18n.resolvedLanguage).toBe('zh-CN');
});
