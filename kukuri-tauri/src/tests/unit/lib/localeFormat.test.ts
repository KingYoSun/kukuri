import { beforeEach, describe, expect, it } from 'vitest';
import i18n from '@/i18n';
import { formatDateTimeByI18n, formatNumberByI18n } from '@/lib/utils/localeFormat';

describe('localeFormat', () => {
  const sampleDate = new Date(Date.UTC(2026, 0, 2, 3, 4, 5));

  beforeEach(async () => {
    await i18n.changeLanguage('ja');
  });

  it('i18n.language に応じて日時フォーマットを切り替える', async () => {
    const options: Intl.DateTimeFormatOptions = {
      timeZone: 'UTC',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      weekday: 'long',
    };

    await i18n.changeLanguage('ja');
    const ja = formatDateTimeByI18n(sampleDate, options);

    await i18n.changeLanguage('en');
    const en = formatDateTimeByI18n(sampleDate, options);

    await i18n.changeLanguage('zh-CN');
    const zhCn = formatDateTimeByI18n(sampleDate, options);

    expect(ja).toBe(new Intl.DateTimeFormat('ja', options).format(sampleDate));
    expect(en).toBe(new Intl.DateTimeFormat('en', options).format(sampleDate));
    expect(zhCn).toBe(new Intl.DateTimeFormat('zh-CN', options).format(sampleDate));
    expect(new Set([ja, en, zhCn]).size).toBeGreaterThan(1);
  });

  it('無効な日時入力の場合は空文字を返す', () => {
    expect(formatDateTimeByI18n('invalid-date')).toBe('');
  });

  it('i18n.language に応じて数値フォーマットを適用する', async () => {
    const value = 1234567.89;
    const options: Intl.NumberFormatOptions = {
      style: 'currency',
      currency: 'JPY',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    };

    await i18n.changeLanguage('ja');
    const ja = formatNumberByI18n(value, options);

    await i18n.changeLanguage('en');
    const en = formatNumberByI18n(value, options);

    expect(ja).toBe(new Intl.NumberFormat('ja', options).format(value));
    expect(en).toBe(new Intl.NumberFormat('en', options).format(value));
    expect(ja).not.toBe(en);
  });

  it('非有限数値の場合は空文字を返す', () => {
    expect(formatNumberByI18n(Number.NaN)).toBe('');
    expect(formatNumberByI18n(Number.POSITIVE_INFINITY)).toBe('');
  });
});
