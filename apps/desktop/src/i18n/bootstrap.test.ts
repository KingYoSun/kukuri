import { afterEach, expect, test, vi } from 'vitest';
import i18n, { DESKTOP_LOCALE_STORAGE_KEY } from './index';
import { initializeDesktopLocale, SYSTEM_LOCALE_TIMEOUT_MS } from './bootstrap';
import { changeDesktopLocale } from './changeLocale';
import { firstSupportedLocale, matchSupportedLocale } from './locale';

afterEach(() => { vi.useRealTimers(); });

test.each([
  ['ja_JP.UTF-8', 'ja'], [' ja-JP ', 'ja'], ['en_US.UTF-8', 'en'],
  ['zh_CN.UTF-8', 'zh-CN'], ['zh-Hans-CN', 'zh-CN'], ['zh-TW', 'zh-CN'],
  ['ja_JP@calendar=japanese', 'ja'], ['jargon', null], ['', null], ['fr-FR', null],
])('matches the locale %s without eagerly falling back', (value, expected) => {
  expect(matchSupportedLocale(value)).toBe(expected);
});

test('an unsupported first candidate does not hide a later supported language', () => {
  expect(firstSupportedLocale(['fr-FR', 'ja-JP', 'en-US'])).toBe('ja');
});

test.each(['ja', 'en', 'zh-CN'] as const)('saved %s takes priority over the OS without IPC', async (locale) => {
  localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, locale);
  const systemLocales = vi.fn(async () => ['ja-JP']);
  expect(await initializeDesktopLocale({systemLocales, navigatorLanguages: ['en-US']})).toBe(locale);
  expect(systemLocales).not.toHaveBeenCalled();
  expect(i18n.resolvedLanguage).toBe(locale);
  expect(document.documentElement.lang).toBe(locale);
});

test.each([null, 'unknown', ''])('fresh or invalid saved value %s uses the OS before navigator', async (stored) => {
  if (stored === null) localStorage.removeItem(DESKTOP_LOCALE_STORAGE_KEY);
  else localStorage.setItem(DESKTOP_LOCALE_STORAGE_KEY, stored);
  await initializeDesktopLocale({systemLocales: async () => ['fr-FR', 'ja_JP.UTF-8'], navigatorLanguages:['en-US']});
  expect(i18n.resolvedLanguage).toBe('ja');
  expect(localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('ja');
});

test('system failure falls back through navigator candidates, then English', async () => {
  localStorage.removeItem(DESKTOP_LOCALE_STORAGE_KEY);
  const systemLocales = vi.fn(async () => { throw new Error('OS unavailable'); });
  expect(await initializeDesktopLocale({systemLocales, navigatorLanguages:['fr', 'zh-Hans']})).toBe('zh-CN');
  localStorage.removeItem(DESKTOP_LOCALE_STORAGE_KEY);
  expect(await initializeDesktopLocale({systemLocales, navigatorLanguages:['fr']})).toBe('en');
});

test('a storage exception does not prevent detection or changing the session language', async () => {
  const storage = {
    getItem: () => { throw new Error('read denied'); },
    setItem: () => { throw new Error('write denied'); },
  } as unknown as Storage;
  expect(await initializeDesktopLocale({storage, systemLocales:async () => ['ja'], navigatorLanguages:[]})).toBe('ja');
  expect(changeDesktopLocale('zh-CN', storage)).toBe(false);
  expect(i18n.resolvedLanguage).toBe('zh-CN');
  expect(document.documentElement.lang).toBe('zh-CN');
});

test('OS timeout completes startup and a late result cannot overwrite a later selection', async () => {
  vi.useFakeTimers();
  localStorage.removeItem(DESKTOP_LOCALE_STORAGE_KEY);
  let complete!: (languages: string[]) => void;
  const boot = initializeDesktopLocale({systemLocales: () => new Promise((resolve) => { complete = resolve; }), navigatorLanguages:['en-US']});
  await vi.advanceTimersByTimeAsync(SYSTEM_LOCALE_TIMEOUT_MS);
  expect(await boot).toBe('en');
  changeDesktopLocale('zh-CN');
  complete(['ja-JP']);
  await vi.runOnlyPendingTimersAsync();
  expect(i18n.resolvedLanguage).toBe('zh-CN');
  expect(localStorage.getItem(DESKTOP_LOCALE_STORAGE_KEY)).toBe('zh-CN');
  expect(vi.getTimerCount()).toBe(0);
});

test('a preference saved while the OS read is pending wins', async () => {
  localStorage.removeItem(DESKTOP_LOCALE_STORAGE_KEY);
  let complete!: (languages: string[]) => void;
  const boot = initializeDesktopLocale({systemLocales: () => new Promise((resolve) => { complete = resolve; })});
  await Promise.resolve();
  changeDesktopLocale('en');
  complete(['ja']);
  expect(await boot).toBe('en');
});
