import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { SUPPORTED_LOCALES } from './locale';
export {
  DESKTOP_LOCALE_STORAGE_KEY, SUPPORTED_LOCALES, normalizeSupportedLocale,
  type SupportedLocale,
} from './locale';

import commonEn from './locales/en/common.json';
import shellEn from './locales/en/shell.json';
import settingsEn from './locales/en/settings.json';
import profileEn from './locales/en/profile.json';
import channelsEn from './locales/en/channels.json';
import liveEn from './locales/en/live.json';
import gameEn from './locales/en/game.json';
import legalEn from './locales/en/legal.json';
import metaverseEn from './locales/en/metaverse.json';
import commonJa from './locales/ja/common.json';
import shellJa from './locales/ja/shell.json';
import settingsJa from './locales/ja/settings.json';
import profileJa from './locales/ja/profile.json';
import channelsJa from './locales/ja/channels.json';
import liveJa from './locales/ja/live.json';
import gameJa from './locales/ja/game.json';
import legalJa from './locales/ja/legal.json';
import metaverseJa from './locales/ja/metaverse.json';
import commonZhCn from './locales/zh-CN/common.json';
import shellZhCn from './locales/zh-CN/shell.json';
import settingsZhCn from './locales/zh-CN/settings.json';
import profileZhCn from './locales/zh-CN/profile.json';
import channelsZhCn from './locales/zh-CN/channels.json';
import liveZhCn from './locales/zh-CN/live.json';
import gameZhCn from './locales/zh-CN/game.json';
import legalZhCn from './locales/zh-CN/legal.json';
import metaverseZhCn from './locales/zh-CN/metaverse.json';

export const resources = {
  en: {
    channels: channelsEn,
    common: commonEn,
    game: gameEn,
    legal: legalEn,
    live: liveEn,
    metaverse: metaverseEn,
    profile: profileEn,
    settings: settingsEn,
    shell: shellEn,
  },
  ja: {
    channels: channelsJa,
    common: commonJa,
    game: gameJa,
    legal: legalJa,
    live: liveJa,
    metaverse: metaverseJa,
    profile: profileJa,
    settings: settingsJa,
    shell: shellJa,
  },
  'zh-CN': {
    channels: channelsZhCn,
    common: commonZhCn,
    game: gameZhCn,
    legal: legalZhCn,
    live: liveZhCn,
    metaverse: metaverseZhCn,
    profile: profileZhCn,
    settings: settingsZhCn,
    shell: shellZhCn,
  },
} as const;

if (!i18n.isInitialized) {
  void i18n
    .use(initReactI18next)
    .init({
      // resourceは全て同梱。import時には検出・保存せず、mainのbootstrapが描画前に決定する。
      lng: 'en',
      initAsync: false,
      resources,
      supportedLngs: [...SUPPORTED_LOCALES],
      fallbackLng: {
        zh: ['zh-CN'],
        default: ['en'],
      },
      defaultNS: 'common',
      fallbackNS: 'common',
      ns: ['common', 'shell', 'settings', 'profile', 'channels', 'live', 'game', 'legal', 'metaverse'],
      react: {
        useSuspense: false,
      },
      interpolation: {
        escapeValue: false,
      },
    });
}

export default i18n;
