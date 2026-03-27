import i18n from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next } from 'react-i18next';

import commonEn from './locales/en/common.json';
import shellEn from './locales/en/shell.json';
import settingsEn from './locales/en/settings.json';
import profileEn from './locales/en/profile.json';
import channelsEn from './locales/en/channels.json';
import liveEn from './locales/en/live.json';
import gameEn from './locales/en/game.json';
import commonJa from './locales/ja/common.json';
import shellJa from './locales/ja/shell.json';
import settingsJa from './locales/ja/settings.json';
import profileJa from './locales/ja/profile.json';
import channelsJa from './locales/ja/channels.json';
import liveJa from './locales/ja/live.json';
import gameJa from './locales/ja/game.json';
import commonZhCn from './locales/zh-CN/common.json';
import shellZhCn from './locales/zh-CN/shell.json';
import settingsZhCn from './locales/zh-CN/settings.json';
import profileZhCn from './locales/zh-CN/profile.json';
import channelsZhCn from './locales/zh-CN/channels.json';
import liveZhCn from './locales/zh-CN/live.json';
import gameZhCn from './locales/zh-CN/game.json';

export const SUPPORTED_LOCALES = ['ja', 'en', 'zh-CN'] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const DESKTOP_LOCALE_STORAGE_KEY = 'kukuri.desktop.locale';

export function normalizeSupportedLocale(value: string | null | undefined): SupportedLocale {
  if (!value) {
    return 'en';
  }

  const normalized = value.toLowerCase();
  if (normalized === 'zh' || normalized.startsWith('zh-')) {
    return 'zh-CN';
  }
  if (normalized.startsWith('ja')) {
    return 'ja';
  }
  if (normalized.startsWith('en')) {
    return 'en';
  }

  return 'en';
}

export const resources = {
  en: {
    channels: channelsEn,
    common: commonEn,
    game: gameEn,
    live: liveEn,
    profile: profileEn,
    settings: settingsEn,
    shell: shellEn,
  },
  ja: {
    channels: channelsJa,
    common: commonJa,
    game: gameJa,
    live: liveJa,
    profile: profileJa,
    settings: settingsJa,
    shell: shellJa,
  },
  'zh-CN': {
    channels: channelsZhCn,
    common: commonZhCn,
    game: gameZhCn,
    live: liveZhCn,
    profile: profileZhCn,
    settings: settingsZhCn,
    shell: shellZhCn,
  },
} as const;

if (!i18n.isInitialized) {
  void i18n
    .use(LanguageDetector)
    .use(initReactI18next)
    .init({
      resources,
      supportedLngs: [...SUPPORTED_LOCALES],
      fallbackLng: {
        zh: ['zh-CN'],
        default: ['en'],
      },
      defaultNS: 'common',
      fallbackNS: 'common',
      ns: ['common', 'shell', 'settings', 'profile', 'channels', 'live', 'game'],
      react: {
        useSuspense: false,
      },
      interpolation: {
        escapeValue: false,
      },
      detection: {
        order: ['localStorage', 'navigator'],
        caches: ['localStorage'],
        lookupLocalStorage: DESKTOP_LOCALE_STORAGE_KEY,
        convertDetectedLanguage: normalizeSupportedLocale,
      },
    });
}

export default i18n;
