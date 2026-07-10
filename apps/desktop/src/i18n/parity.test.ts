// i18n キーパリティの凍結テスト(WP-S4 T5)。
//
// 全ロケールが en と同一のキー集合を持つことを固定する。fail した場合は
// 追加したキーを全ロケールへ反映すること(テストを緩めない)。
//
// 既知の限界:
// - locales/ に新しい JSON を置いても index.ts に import されなければ検出外
//   (ns 一致テストが import 漏れを部分的に緩和する)。
// - 将来 legal.json の段落構造(配列)をロケール間で意図的に変える場合は、
//   legal のみ配列を葉扱いにする設計変更が必要。
// - 複数形サフィックス(_one/_other 等)が導入された場合、CLDR カテゴリ差に
//   よる正当なキー差を誤検出し得る(現状は全ロケール未使用)。
import { describe, expect, test } from 'vitest';

import { SUPPORTED_LOCALES, resources } from './index';

const BASE_LOCALE = 'en';
const NAMESPACES = Object.keys(resources[BASE_LOCALE]).sort();
const OTHER_LOCALES = SUPPORTED_LOCALES.filter((locale) => locale !== BASE_LOCALE);

type Json = string | number | boolean | null | Json[] | { [key: string]: Json };

// オブジェクトは再帰、配列は index 付き([i])で再帰、それ以外は葉。
function flattenKeys(value: Json, prefix: string): string[] {
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => flattenKeys(item, `${prefix}[${index}]`));
  }
  if (typeof value === 'object' && value !== null) {
    return Object.entries(value).flatMap(([key, child]) =>
      flattenKeys(child, prefix ? `${prefix}.${key}` : key)
    );
  }
  return [prefix];
}

function flattenLeaves(value: Json, prefix: string): Array<[string, string]> {
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => flattenLeaves(item, `${prefix}[${index}]`));
  }
  if (typeof value === 'object' && value !== null) {
    return Object.entries(value).flatMap(([key, child]) =>
      flattenLeaves(child, prefix ? `${prefix}.${key}` : key)
    );
  }
  return typeof value === 'string' ? [[prefix, value]] : [];
}

function placeholders(text: string): string[] {
  return [...text.matchAll(/\{\{(\w+)\}\}/g)].map((match) => match[1]).sort();
}

test('every locale exposes the same namespaces as en', () => {
  for (const locale of OTHER_LOCALES) {
    expect(Object.keys(resources[locale]).sort()).toEqual(NAMESPACES);
  }
});

describe.each(OTHER_LOCALES)('locale %s', (locale) => {
  test.each(NAMESPACES)('namespace %s has the same key set as en', (namespace) => {
    const enKeys = new Set(
      flattenKeys(resources[BASE_LOCALE][namespace as keyof (typeof resources)['en']] as Json, '')
    );
    const localeKeys = new Set(
      flattenKeys(resources[locale][namespace as keyof (typeof resources)['en']] as Json, '')
    );
    const missing = [...enKeys].filter((key) => !localeKeys.has(key)).sort();
    const extra = [...localeKeys].filter((key) => !enKeys.has(key)).sort();
    expect(missing).toEqual([]);
    expect(extra).toEqual([]);
  });

  test.each(NAMESPACES)('namespace %s keeps en placeholders', (namespace) => {
    const enLeaves = new Map(
      flattenLeaves(resources[BASE_LOCALE][namespace as keyof (typeof resources)['en']] as Json, '')
    );
    const localeLeaves = new Map(
      flattenLeaves(resources[locale][namespace as keyof (typeof resources)['en']] as Json, '')
    );
    for (const [key, enValue] of enLeaves) {
      const localeValue = localeLeaves.get(key);
      if (localeValue === undefined) continue; // キー欠落は上のテストが報告する
      expect(placeholders(localeValue), `${namespace}:${key}`).toEqual(placeholders(enValue));
    }
  });
});

test('i18n init namespaces match the bundled resources', () => {
  // index.ts の ns 配列と resources のキーがずれると、バンドル済みなのに
  // 参照されない(またはその逆の)名前空間が生まれる。
  const initNamespaces = [
    'common',
    'shell',
    'settings',
    'profile',
    'channels',
    'live',
    'game',
    'legal',
    'metaverse',
  ]
    .slice()
    .sort();
  expect(NAMESPACES).toEqual(initNamespaces);
});
