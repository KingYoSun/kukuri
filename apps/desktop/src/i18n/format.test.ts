import { expect, test } from 'vitest';
import { formatLocalizedTime, formatPostDateTime } from './format';

test.each(['ja', 'en', 'zh-CN'])('post date retains year, day and seconds across a year boundary (%s)', locale => {
  const before = new Date(2025, 11, 31, 23, 59, 59);
  const after = new Date(2026, 0, 1, 0, 0, 1);
  expect(formatPostDateTime(before, locale)).toContain('2025');
  expect(formatPostDateTime(before, locale)).toContain('31');
  expect(formatPostDateTime(before, locale)).toContain('59:59');
  expect(formatPostDateTime(after, locale)).toContain('2026');
  expect(formatPostDateTime(after, locale)).toContain('01');
  expect(formatPostDateTime(after, locale)).toContain('00:01');
  expect(formatLocalizedTime(before, locale)).not.toContain('2025');
});
