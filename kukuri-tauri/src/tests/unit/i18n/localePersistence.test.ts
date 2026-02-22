import { describe, expect, it, vi } from 'vitest';

import {
  LEGACY_LOCALE_STORAGE_KEYS,
  LOCALE_STORAGE_KEY,
  migrateLegacyLocaleStorage,
  normalizeLocale,
  resolveStoredLocale,
} from '@/i18n';

const createStorage = (initial: Record<string, string | null> = {}) => {
  const values = new Map<string, string>();
  Object.entries(initial).forEach(([key, value]) => {
    if (typeof value === 'string') {
      values.set(key, value);
    }
  });

  const storage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: vi.fn((key: string, value: string) => {
      values.set(key, value);
    }),
  };

  return { storage, values };
};

describe('locale persistence helpers', () => {
  it('ブラウザロケールを対応ロケールへ正規化する', () => {
    expect(normalizeLocale('ja-JP')).toBe('ja');
    expect(normalizeLocale('zh-TW')).toBe('zh-CN');
    expect(normalizeLocale('en-US')).toBe('en');
  });

  it('新キーに保存されたロケールを優先復元する', () => {
    const { storage } = createStorage({
      [LOCALE_STORAGE_KEY]: 'zh-Hans',
      [LEGACY_LOCALE_STORAGE_KEYS[0]]: 'ja-JP',
    });

    expect(resolveStoredLocale(storage)).toEqual({
      locale: 'zh-CN',
      source: 'primary',
    });
  });

  it('新キーがない場合は旧キーから復元する', () => {
    const { storage } = createStorage({
      [LEGACY_LOCALE_STORAGE_KEYS[0]]: 'ja-JP',
    });

    expect(resolveStoredLocale(storage)).toEqual({
      locale: 'ja',
      source: 'legacy',
    });
  });

  it('旧キーのJSON形式ロケール値を復元できる', () => {
    const { storage } = createStorage({
      [LEGACY_LOCALE_STORAGE_KEYS[1]]: '{"lng":"ja-JP"}',
    });

    expect(resolveStoredLocale(storage)).toEqual({
      locale: 'ja',
      source: 'legacy',
    });
  });

  it('旧キー復元時は新キーへ移行保存する', () => {
    const { storage, values } = createStorage({
      [LEGACY_LOCALE_STORAGE_KEYS[0]]: 'en-US',
    });

    const locale = migrateLegacyLocaleStorage(storage);

    expect(locale).toBe('en');
    expect(storage.setItem).toHaveBeenCalledWith(LOCALE_STORAGE_KEY, 'en');
    expect(values.get(LOCALE_STORAGE_KEY)).toBe('en');
  });

  it('既に新キーがある場合は移行書き込みしない', () => {
    const { storage } = createStorage({
      [LOCALE_STORAGE_KEY]: 'ja',
      [LEGACY_LOCALE_STORAGE_KEYS[0]]: 'en-US',
    });

    const locale = migrateLegacyLocaleStorage(storage);

    expect(locale).toBe('ja');
    expect(storage.setItem).not.toHaveBeenCalled();
  });
});
