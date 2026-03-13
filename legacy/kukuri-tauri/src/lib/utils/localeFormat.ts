import { getCurrentLocale } from '@/i18n';

const DEFAULT_DATE_TIME_OPTIONS: Intl.DateTimeFormatOptions = {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
};

export const formatDateTimeByI18n = (
  value: Date | number | string,
  options: Intl.DateTimeFormatOptions = DEFAULT_DATE_TIME_OPTIONS,
) => {
  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return '';
  }

  return new Intl.DateTimeFormat(getCurrentLocale(), options).format(date);
};

export const formatNumberByI18n = (value: number, options?: Intl.NumberFormatOptions) => {
  if (!Number.isFinite(value)) {
    return '';
  }

  return new Intl.NumberFormat(getCurrentLocale(), options).format(value);
};
