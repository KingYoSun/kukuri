import { useEffect, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { NotificationView } from '@/lib/api';
import { isTauriRuntime } from '@/lib/releaseReadiness';

type ActivationPayload = {
  notification_id: string;
};

/**
 * Listens for OS toast clicks emitted by the `show_os_notification` Rust command
 * and opens the related notification target via the existing in-app handler.
 *
 * The Rust side already focuses the window; here we just resolve the activated
 * id back to a notification and reuse `handleOpenNotification`.
 */
export function useOsNotificationActivation(
  notifications: NotificationView[],
  onActivate: (notification: NotificationView) => void
): void {
  const notificationsRef = useRef<NotificationView[]>(notifications);
  const onActivateRef = useRef(onActivate);

  useEffect(() => {
    notificationsRef.current = notifications;
  }, [notifications]);

  useEffect(() => {
    onActivateRef.current = onActivate;
  }, [onActivate]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    void (async () => {
      const dispose = await listen<ActivationPayload>('os-notification://activated', (event) => {
        const notificationId = event.payload?.notification_id;
        if (!notificationId) {
          return;
        }
        const notification = notificationsRef.current.find(
          (candidate) => candidate.notification_id === notificationId
        );
        if (notification) {
          onActivateRef.current(notification);
        }
      });
      if (cancelled) {
        dispose();
        return;
      }
      unlisten = dispose;
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
}
