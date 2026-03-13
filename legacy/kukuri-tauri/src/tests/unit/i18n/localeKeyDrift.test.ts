import { describe, it, expect } from 'vitest';
import enLocale from '@/locales/en.json';
import jaLocale from '@/locales/ja.json';
import zhCNLocale from '@/locales/zh-CN.json';

type LocaleJson = Record<string, unknown>;

const flattenKeys = (value: unknown, prefix = ''): string[] => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    return prefix ? [prefix] : [];
  }

  const entries = Object.entries(value);
  if (entries.length === 0) {
    return prefix ? [prefix] : [];
  }

  return entries.flatMap(([key, child]) => {
    const childPrefix = prefix ? `${prefix}.${key}` : key;
    return flattenKeys(child, childPrefix);
  });
};

describe('locale key drift guard', () => {
  it('ja/en/zh-CN should have the same key set', () => {
    const locales: Record<string, LocaleJson> = {
      ja: jaLocale as LocaleJson,
      en: enLocale as LocaleJson,
      'zh-CN': zhCNLocale as LocaleJson,
    };

    const keySets: Record<string, Set<string>> = {};
    for (const [locale, json] of Object.entries(locales)) {
      keySets[locale] = new Set(flattenKeys(json));
    }

    const unionKeys = new Set<string>();
    for (const keySet of Object.values(keySets)) {
      for (const key of keySet) {
        unionKeys.add(key);
      }
    }

    for (const [locale, keySet] of Object.entries(keySets)) {
      const missingKeys = Array.from(unionKeys)
        .filter((key) => !keySet.has(key))
        .sort();
      expect(missingKeys, `[${locale}] missing keys: ${missingKeys.join(', ')}`).toEqual([]);
    }
  });
});
