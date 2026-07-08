import { type DesktopApi, type NotificationStatusView } from '@/lib/api';

import { cloneNotification } from '../desktopMockModel';
import { type MockRuntime } from '../mockRuntime';

type NotificationsMock = Pick<
  DesktopApi,
  'listNotifications' | 'markNotificationRead' | 'markAllNotificationsRead' | 'getNotificationStatus'
>;

export function createNotificationsMock(runtime: MockRuntime): NotificationsMock {
  return {
    async listNotifications() {
      return runtime.notifications.map(cloneNotification);
    },
    async markNotificationRead(notificationId) {
      runtime.notifications = runtime.notifications.map((notification) =>
        notification.notification_id === notificationId && !notification.read_at
          ? { ...notification, read_at: Date.now() }
          : notification
      );
      return {
        unread_count: runtime.notifications.filter((notification) => !notification.read_at).length,
      } satisfies NotificationStatusView;
    },
    async markAllNotificationsRead() {
      const readAt = Date.now();
      runtime.notifications = runtime.notifications.map((notification) =>
        notification.read_at ? notification : { ...notification, read_at: readAt }
      );
      return {
        unread_count: 0,
      } satisfies NotificationStatusView;
    },
    async getNotificationStatus() {
      return {
        unread_count: runtime.notifications.filter((notification) => !notification.read_at).length,
      } satisfies NotificationStatusView;
    },
  };
}
