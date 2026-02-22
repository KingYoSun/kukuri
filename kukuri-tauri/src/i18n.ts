import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { ja, enUS, zhCN } from 'date-fns/locale';

import jaLocale from './locales/ja.json';
import en from './locales/en.json';
import zhCNLocale from './locales/zh-CN.json';

export const SUPPORTED_LOCALES = ['ja', 'en', 'zh-CN'] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];
export const LOCALE_STORAGE_KEY = 'kukuri-locale';
export const LEGACY_LOCALE_STORAGE_KEYS = ['i18nextLng', 'kukuri-language'] as const;

const dateFnsLocaleMap = { ja, en: enUS, 'zh-CN': zhCN } as const;
type StorageReader = Pick<Storage, 'getItem'>;
type StorageWriter = Pick<Storage, 'setItem'>;
type LocaleStorageSource = 'primary' | 'legacy' | 'none';

const resources = {
  ja: { translation: jaLocale },
  en: { translation: en },
  'zh-CN': { translation: zhCNLocale },
};

export const normalizeLocale = (lng: string): SupportedLocale => {
  const lower = lng.toLowerCase();
  if (lower.startsWith('ja')) return 'ja';
  if (lower.startsWith('zh')) return 'zh-CN';
  return 'en';
};

const resolveLocaleCandidate = (candidate: unknown): SupportedLocale | null => {
  if (typeof candidate !== 'string' || candidate.trim() === '') {
    return null;
  }
  return normalizeLocale(candidate);
};

const resolveLocaleFromRawStorageValue = (rawValue: string | null): SupportedLocale | null => {
  if (rawValue == null || rawValue === '') {
    return null;
  }

  const directLocale = resolveLocaleCandidate(rawValue);
  if (directLocale) {
    return directLocale;
  }

  try {
    const parsed = JSON.parse(rawValue) as unknown;
    if (typeof parsed === 'object' && parsed !== null) {
      const record = parsed as Record<string, unknown>;
      return (
        resolveLocaleCandidate(record.lng) ??
        resolveLocaleCandidate(record.language) ??
        resolveLocaleCandidate(record.locale) ??
        resolveLocaleCandidate(record.value)
      );
    }
    return resolveLocaleCandidate(parsed);
  } catch {
    return null;
  }
};

export const resolveStoredLocale = (
  storage: StorageReader | null,
): { locale: SupportedLocale | null; source: LocaleStorageSource } => {
  if (!storage) {
    return { locale: null, source: 'none' };
  }

  const primaryLocale = resolveLocaleFromRawStorageValue(storage.getItem(LOCALE_STORAGE_KEY));
  if (primaryLocale) {
    return { locale: primaryLocale, source: 'primary' };
  }

  for (const legacyKey of LEGACY_LOCALE_STORAGE_KEYS) {
    const legacyLocale = resolveLocaleFromRawStorageValue(storage.getItem(legacyKey));
    if (legacyLocale) {
      return { locale: legacyLocale, source: 'legacy' };
    }
  }

  return { locale: null, source: 'none' };
};

export const persistLocale = (
  locale: SupportedLocale,
  storage: StorageWriter | null = typeof window !== 'undefined' ? window.localStorage : null,
): void => {
  if (!storage) {
    return;
  }
  storage.setItem(LOCALE_STORAGE_KEY, locale);
};

export const migrateLegacyLocaleStorage = (
  storage: StorageReader | (StorageReader & StorageWriter) | null,
): SupportedLocale | null => {
  const { locale, source } = resolveStoredLocale(storage);
  if (!locale) {
    return null;
  }
  if (source === 'legacy' && storage && 'setItem' in storage) {
    storage.setItem(LOCALE_STORAGE_KEY, locale);
  }
  return locale;
};

const mapNavigatorToLocale = (lng: string): SupportedLocale => normalizeLocale(lng);
const browserStorage = typeof window !== 'undefined' ? window.localStorage : null;
const startupLocale = migrateLegacyLocaleStorage(browserStorage);

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    lng: startupLocale ?? undefined,
    fallbackLng: 'en',
    supportedLngs: SUPPORTED_LOCALES,
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: LOCALE_STORAGE_KEY,
      convertDetectedLanguage: mapNavigatorToLocale,
    },
  });

/** Normalize i18n.language (e.g. "en-US") to our SupportedLocale */
export function getCurrentLocale(): SupportedLocale {
  return normalizeLocale(i18n.language ?? 'en');
}

/** Get date-fns locale based on current i18n locale */
export function getDateFnsLocale() {
  const locale = getCurrentLocale();
  return dateFnsLocaleMap[locale] ?? enUS;
}

export { i18n };
export default i18n;
