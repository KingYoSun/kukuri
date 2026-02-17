import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { ja, enUS, zhCN } from 'date-fns/locale';

import jaLocale from './locales/ja.json';
import en from './locales/en.json';
import zhCNLocale from './locales/zh-CN.json';

export const SUPPORTED_LOCALES = ['ja', 'en', 'zh-CN'] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

const dateFnsLocaleMap = { ja, en: enUS, 'zh-CN': zhCN } as const;

const resources = {
  ja: { translation: jaLocale },
  en: { translation: en },
  'zh-CN': { translation: zhCNLocale },
};

const mapNavigatorToLocale = (lng: string): SupportedLocale => {
  const lower = lng.toLowerCase();
  if (lower.startsWith('ja')) return 'ja';
  if (lower.startsWith('zh')) return 'zh-CN';
  return 'en';
};

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: 'en',
    supportedLngs: SUPPORTED_LOCALES,
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: 'kukuri-locale',
      convertDetectedLanguage: mapNavigatorToLocale,
    },
  });

/** Normalize i18n.language (e.g. "en-US") to our SupportedLocale */
export function getCurrentLocale(): SupportedLocale {
  const lng = i18n.language?.toLowerCase() ?? '';
  if (lng.startsWith('ja')) return 'ja';
  if (lng.startsWith('zh')) return 'zh-CN';
  return 'en';
}

/** Get date-fns locale based on current i18n locale */
export function getDateFnsLocale() {
  const locale = getCurrentLocale();
  return dateFnsLocaleMap[locale] ?? enUS;
}

export { i18n };
export default i18n;
