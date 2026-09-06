import { describe, expect, test } from 'vitest';

import { parseOsNotificationActivationLink } from './osNotificationActivationLink';

describe('notification activation links', () => {
  test('decodes only the opaque notification ID', () => {
    expect(parseOsNotificationActivationLink('kukuri://notification?id=notification%3Aowner%3Areply%3Aabc'))
      .toBe('notification:owner:reply:abc');
    expect(parseOsNotificationActivationLink('kukuri://notification?id=%E9%80%9A%E7%9F%A5+id'))
      .toBe('通知 id');
  });

  test('accepts the root slash added by Windows protocol activation', () => {
    expect(parseOsNotificationActivationLink('kukuri://notification/?id=notification%3Aowner%3Areply%3Aabc'))
      .toBe('notification:owner:reply:abc');
    expect(parseOsNotificationActivationLink('kukuri://notification/?id=%E9%80%9A%E7%9F%A5+id'))
      .toBe('通知 id');
  });

  test.each([
    'https://notification?id=x', 'kukuri://notification/extra?id=x',
    'kukuri://notification//?id=x', 'kukuri://notification/%2F?id=x',
    'kukuri://notification.evil/?id=x', 'kukuri://notification/?id=x&profile=other',
    'kukuri://notification/?id=x#fragment', 'kukuri://notification/?id=%00',
    'kukuri://user@notification?id=x', 'kukuri://notification:80?id=x',
    'kukuri://notification?id=x#fragment', 'kukuri://notification?id=x&id=y',
    'kukuri://notification?id=x&profile=other', 'kukuri://notification?id=x&body=secret',
    'kukuri://notification?id=', 'kukuri://notification?id=%ZZ',
    'kukuri://notification?id=%00', 'kukuri://notification?id=%0A',
    'kukuri://notification?id=%ED%A0%80', `kukuri://notification?id=${'a'.repeat(1025)}`,
    `kukuri://notification?id=${'%61'.repeat(1500)}`,
  ])('rejects malformed or expanded activation input: %s', (uri) => {
    expect(parseOsNotificationActivationLink(uri)).toBeNull();
  });
});
