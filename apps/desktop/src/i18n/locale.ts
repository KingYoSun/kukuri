export const SUPPORTED_LOCALES = ['ja', 'en', 'zh-CN'] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];
export const DESKTOP_LOCALE_STORAGE_KEY = 'kukuri.desktop.locale';

// 未対応をenへ変換する前に候補を全て調べる。POSIX localeのencoding/modifierは表示言語ではない。
export function matchSupportedLocale(value: unknown): SupportedLocale | null {
  if (typeof value !== 'string') return null;
  const language = value.trim().split(/[.@]/, 1)[0].replaceAll('_', '-').toLowerCase();
  if (/^ja(?:-|$)/.test(language)) return 'ja';
  if (/^en(?:-|$)/.test(language)) return 'en';
  if (/^zh(?:-|$)/.test(language)) return 'zh-CN';
  return null;
}

export function normalizeSupportedLocale(value: string | null | undefined): SupportedLocale {
  return matchSupportedLocale(value) ?? 'en';
}

export function firstSupportedLocale(candidates: readonly string[]): SupportedLocale | null {
  for (const candidate of candidates) {
    const locale = matchSupportedLocale(candidate);
    if (locale !== null) return locale;
  }
  return null;
}

export function desktopLocaleStorage(): Storage | null {
  try {
    return typeof window === 'undefined' ? null : window.localStorage;
  } catch {
    return null;
  }
}

export function readStoredLocale(storage: Storage | null): SupportedLocale | null {
  try {
    return matchSupportedLocale(storage?.getItem(DESKTOP_LOCALE_STORAGE_KEY));
  } catch {
    return null;
  }
}
