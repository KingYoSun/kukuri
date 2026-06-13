import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

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
    // Route through the Tauri backend `notify` command directly. The
    // `@tauri-apps/plugin-notification` JS helpers go through the WebView2 Web
    // Notification API, which does not produce real Windows toasts and reports a
    // volatile permission state (see issue #313). The Rust backend grants
    // permission unconditionally on desktop and emits a proper OS toast.
    void (async () => {
      for (const notification of unseen) {
        if (cancelled) {
          break;
        }
        try {
          await invoke('plugin:notification|notify', {
            options: {
              id: nextOsNotificationId(notification.notification_id),
              title: notificationTitle(notification),
              body: notificationBody(notification, settings),
              silent: settings.quietMode,
            },
          });
          seenNotificationIdsRef.current.add(notification.notification_id);
        } catch {
          // best effort OS notification
        }
      }
      if (!cancelled) {
        writeSeenOsNotificationIds(seenNotificationIdsRef.current);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [localAuthorPubkey, notifications]);
}
