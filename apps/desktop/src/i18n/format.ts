import i18n, { normalizeSupportedLocale, type SupportedLocale } from './index';

export function getResolvedLocale(locale?: string | null): SupportedLocale {
  return normalizeSupportedLocale(locale ?? i18n.resolvedLanguage ?? i18n.language);
}

export function formatLocalizedNumber(value: number, locale?: string | null): string {
  return new Intl.NumberFormat(getResolvedLocale(locale)).format(value);
}

export function formatLocalizedTime(
  value: Date | number | string,
  locale?: string | null
): string {
  const date = value instanceof Date ? value : new Date(value);
  return new Intl.DateTimeFormat(getResolvedLocale(locale), {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  }).format(date);
}

/** 時:分だけの短い時刻。秒は表示しない（#956 更新確認の完了時刻）。 */
export function formatLocalizedClockTime(
  value: Date | number | string,
  locale?: string | null
): string {
  const date = value instanceof Date ? value : new Date(value);
  return new Intl.DateTimeFormat(getResolvedLocale(locale), {
    hour: 'numeric',
    minute: '2-digit',
  }).format(date);
}

export function formatLocalizedDateTime(
  value: Date | number | string,
  locale?: string | null
): string {
  const date = value instanceof Date ? value : new Date(value);
  return new Intl.DateTimeFormat(getResolvedLocale(locale), {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}

export function formatLocalizedBytes(value: number, locale?: string | null): string {
  const resolvedLocale = getResolvedLocale(locale);
  if (value >= 1024 * 1024) {
    return `${new Intl.NumberFormat(resolvedLocale, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(value / (1024 * 1024))} MB`;
  }
  if (value >= 1024) {
    return `${new Intl.NumberFormat(resolvedLocale, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(value / 1024)} KB`;
  }

  return `${new Intl.NumberFormat(resolvedLocale).format(value)} B`;
}
