import { getSystemLocales } from '@/lib/api/systemLocale';
import { changeDesktopLocale } from './changeLocale';
import { desktopLocaleStorage, firstSupportedLocale, readStoredLocale } from './locale';

export const SYSTEM_LOCALE_TIMEOUT_MS = 1500;

type LocaleBootstrapOptions = {
  storage?: Storage | null;
  systemLocales?: () => Promise<string[]>;
  navigatorLanguages?: readonly string[];
};

/** OS取得は一回だけ。期限後の結果には副作用がなく、以後の明示選択を上書きしない。 */
export async function initializeDesktopLocale({
  storage = desktopLocaleStorage(),
  systemLocales = getSystemLocales,
  navigatorLanguages = typeof navigator === 'undefined'
    ? [] : [...navigator.languages, navigator.language],
}: LocaleBootstrapOptions = {}) {
  const stored = readStoredLocale(storage);
  if (stored !== null) {
    changeDesktopLocale(stored, storage);
    return stored;
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  let detected: string[];
  try {
    detected = await Promise.race([
      Promise.resolve().then(systemLocales).catch(() => []),
      new Promise<string[]>((resolve) => {
        timer = setTimeout(() => resolve([]), SYSTEM_LOCALE_TIMEOUT_MS);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
  const locale = readStoredLocale(storage)
    ?? firstSupportedLocale(Array.isArray(detected) ? detected : [])
    ?? firstSupportedLocale(navigatorLanguages)
    ?? 'en';
  changeDesktopLocale(locale, storage);
  return locale;
}
