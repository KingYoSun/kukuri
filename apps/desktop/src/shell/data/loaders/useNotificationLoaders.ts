import { startTransition, useCallback } from 'react';

import type { DesktopApi, NotificationView } from '@/lib/api';
import type { ShellChromeProjection } from '@/components/shell/types';
import { messageFromError } from '@/shell/presentation';
import { useDesktopShellFieldSetter } from '@/shell/store';

export type LoadNotificationsSection = (options?: { markAsRead?: boolean }) => Promise<void>;

type NotificationLoaderArgs = {
  api: DesktopApi;
  activePrimarySection: ShellChromeProjection['activePrimarySection'];
  translate: (key: string, options?: Record<string, unknown>) => string;
};

// badgeとinboxの異なる取得・失敗・既読policyを同じ責務内で維持する。
// 呼出時期、interval/eventの登録・解除はeffects側が所有する。
export function useNotificationLoaders({ api, activePrimarySection, translate }: NotificationLoaderArgs) {
  const setNotificationStatus = useDesktopShellFieldSetter('notificationStatus');
  const setNotifications = useDesktopShellFieldSetter('notifications');
  const setNotificationAutoReadError = useDesktopShellFieldSetter('notificationAutoReadError');
  const setNotificationPanelState = useDesktopShellFieldSetter('notificationPanelState');

  const refreshNotificationStatus = useCallback(async () => {
    if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
      return;
    }
    try {
      const status = await api.getNotificationStatus();
      setNotificationStatus(status);
      if (
        status.unread_count > 0 &&
        activePrimarySection !== 'notifications'
      ) {
        const notificationItems = await api.listNotifications();
        startTransition(() => {
          setNotifications(notificationItems);
        });
      }
    } catch {
      // best effort badge refresh
    }
  }, [api, setNotificationStatus, setNotifications, activePrimarySection]);

  const loadNotificationsSection = useCallback(
    async (options: { markAsRead?: boolean } = {}) => {
      const markAsRead = options.markAsRead ?? true;
      try {
        const [status, notificationItems] = await Promise.all([
          api.getNotificationStatus(),
          api.listNotifications(),
        ]);
        let nextNotifications: NotificationView[] = notificationItems;
        let nextStatus = status;
        if (markAsRead && notificationItems.some((notification) => !notification.read_at)) {
          try {
            nextStatus = await api.markAllNotificationsRead();
            const readAt = Date.now();
            nextNotifications = notificationItems.map((notification) =>
              notification.read_at ? notification : { ...notification, read_at: readAt }
            );
            setNotificationAutoReadError(null);
          } catch (notificationReadError) {
            setNotificationAutoReadError(
              messageFromError(
                notificationReadError,
                translate('shell:notifications.errors.failedAutoRead')
              )
            );
          }
        }
        startTransition(() => {
          setNotificationStatus(nextStatus);
          setNotifications(nextNotifications);
          setNotificationPanelState({ status: 'ready', error: null });
        });
      } catch (error) {
        setNotificationPanelState({
          status: 'error',
          error: messageFromError(
            error,
            translate('shell:notifications.errors.failedToLoad')
          ),
        });
      }
    },
    [
      api,
      setNotificationAutoReadError,
      setNotificationPanelState,
      setNotifications,
      setNotificationStatus,
      translate,
    ]
  );

  return { refreshNotificationStatus, loadNotificationsSection };
}
