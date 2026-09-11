import { describe, expect, test } from 'vitest';

import {
  formatOsNotificationPermission,
  formatUpdateStatus,
} from './releasePanelCopy';

describe('ReleasePanel localized status copy', () => {
  test('maps update and Windows notification permission states to Japanese copy', () => {
    const translations: Record<string, string> = {
      'settings:release.update.statuses.up_to_date': '最新です',
      'settings:release.update.statuses.ready_to_restart': '再起動待ち',
      'settings:release.update.statuses.installing': '更新を適用中',
      'settings:notifications.osNotifications.permissions.granted': '許可済み',
      'settings:notifications.osNotifications.permissions.denied': '拒否済み',
      'settings:notifications.osNotifications.permissions.available': 'サービスに接続済み',
    };
    const t = (key: string) => translations[key] ?? key;

    expect(formatUpdateStatus('up_to_date', t)).toBe('最新です');
    expect(formatUpdateStatus('ready_to_restart', t)).toBe('再起動待ち');
    expect(formatUpdateStatus('installing', t)).toBe('更新を適用中');
    expect(formatOsNotificationPermission('granted', t)).toBe('許可済み');
    expect(formatOsNotificationPermission('denied', t)).toBe('拒否済み');
    expect(formatOsNotificationPermission('available', t)).toBe('サービスに接続済み');
  });
});
