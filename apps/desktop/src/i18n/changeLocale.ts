import i18n from './index';
import { DESKTOP_LOCALE_STORAGE_KEY, desktopLocaleStorage, type SupportedLocale } from './locale';

/** 同梱resourceを同期反映する。保存できなくてもsession内の言語は切り替える。 */
export function changeDesktopLocale(
  locale: SupportedLocale,
  storage: Storage | null = desktopLocaleStorage(),
): boolean {
  void i18n.changeLanguage(locale);
  if (typeof document !== 'undefined') document.documentElement.lang = locale;
  try {
    if (!storage) return false;
    storage.setItem(DESKTOP_LOCALE_STORAGE_KEY, locale);
    return true;
  } catch {
    return false;
  }
}
