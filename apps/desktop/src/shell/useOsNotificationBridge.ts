import { useEffect, useRef } from 'react';

import type { NotificationView } from '@/lib/api';
import {
  isTauriRuntime,
  loadOsNotificationSettings,
  nextOsNotificationId,
  notificationBody,
  notificationTitle,
  OS_NOTIFICATION_SETTINGS_STORAGE_KEY,
  readSeenOsNotificationIds,
  shouldSendOsNotification,
  writeSeenOsNotificationIds,
} from '@/lib/releaseReadiness';

export function useOsNotificationBridge(
  notifications: NotificationView[],
  localAuthorPubkey: string
): void {
  const seenNotificationIdsRef = useRef<Set<string>>(readSeenOsNotificationIds());

  useEffect(() => {
    const handleSettingsChange = () => {
      seenNotificationIdsRef.current = readSeenOsNotificationIds();
    };
    window.addEventListener(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, handleSettingsChange);
    return () => {
      window.removeEventListener(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, handleSettingsChange);
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    const settings = loadOsNotificationSettings();
    const unseen = notifications.filter(
      (notification) =>
        !seenNotificationIdsRef.current.has(notification.notification_id) &&
        shouldSendOsNotification(notification, settings, localAuthorPubkey)
    );
    if (unseen.length === 0) {
      return;
    }

    let cancelled = false;
    void import('@tauri-apps/plugin-notification').then(async (plugin) => {
      const granted = await plugin.isPermissionGranted();
      if (!granted || cancelled) {
        return;
      }
      for (const notification of unseen) {
        plugin.sendNotification({
          id: nextOsNotificationId(notification.notification_id),
          title: notificationTitle(notification),
          body: notificationBody(notification, settings),
          silent: settings.quietMode,
          autoCancel: true,
          extra: {
            notificationId: notification.notification_id,
            topicId: notification.topic_id ?? '',
            dmId: notification.dm_id ?? '',
          },
        });
        seenNotificationIdsRef.current.add(notification.notification_id);
      }
      writeSeenOsNotificationIds(seenNotificationIdsRef.current);
    });

    return () => {
      cancelled = true;
    };
  }, [localAuthorPubkey, notifications]);
}
