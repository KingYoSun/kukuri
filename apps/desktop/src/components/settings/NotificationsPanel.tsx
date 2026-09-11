import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import type { OsNotificationSettings } from '@/lib/releaseReadiness';
import { useOsNotificationPermission, useOsNotificationSettings } from '@/lib/useOsNotificationSettings';

import { formatOsNotificationPermission } from './releasePanelCopy';
import { SettingsActionRow } from './SettingsActionRow';

const OS_NOTIFICATION_SETTING_KEYS: Array<keyof OsNotificationSettings> = [
  'enabled',
  'directMessages',
  'mentionsAndReplies',
  'followsAndReposts',
  'quietMode',
  'previewBody',
];

// #962: 通知の受信設定の正本。以前はリリース section 最下部にあり、通知一覧から辿れなかった。
export function NotificationsPanel() {
  const { t } = useTranslation(['settings']);
  const [settings, updateSettings] = useOsNotificationSettings();
  const permission = useOsNotificationPermission();

  const requestPermission = async () => {
    const result = await permission.request();
    if (result === 'granted') {
      updateSettings({ enabled: true });
    }
  };

  return (
    <Card className='min-w-0 space-y-5'>
      <CardHeader>
        <h3>{t('settings:notifications.title')}</h3>
        <small>{t('settings:notifications.summary')}</small>
      </CardHeader>

      <section className='min-w-0 space-y-3'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('settings:notifications.osNotifications.title')}
        </h4>
        <Notice>
          {t('settings:notifications.osNotifications.permission', {
            permission: formatOsNotificationPermission(permission.permission, t),
          })}
        </Notice>
        <div className='grid gap-3 sm:grid-cols-2'>
          {OS_NOTIFICATION_SETTING_KEYS.map((key) => (
            <label
              key={key}
              className='flex min-w-0 items-center gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] px-4 py-3 text-sm text-foreground'
            >
              <input
                type='checkbox'
                checked={Boolean(settings[key])}
                onChange={(event) => updateSettings({ [key]: event.currentTarget.checked })}
              />
              <span>{t(`settings:notifications.osNotifications.${key}`)}</span>
            </label>
          ))}
        </div>
        <SettingsActionRow>
          <Button
            variant='secondary'
            type='button'
            disabled={permission.checking}
            aria-busy={permission.checking}
            onClick={() => void requestPermission()}
          >
            {t(
              permission.checking
                ? 'settings:notifications.osNotifications.checking'
                : 'settings:notifications.osNotifications.requestPermission'
            )}
          </Button>
        </SettingsActionRow>
      </section>
    </Card>
  );
}
