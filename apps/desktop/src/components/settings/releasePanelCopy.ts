import { type UpdateState } from '@/lib/releaseReadiness';

type Translate = (key: string) => string;

export function formatUpdateStatus(status: UpdateState['status'], t: Translate): string {
  return t(`settings:release.update.statuses.${status}`);
}

export function formatOsNotificationPermission(permission: string, t: Translate): string {
  const knownPermission = ['granted', 'denied', 'prompt', 'unknown', 'unavailable'].includes(
    permission
  )
    ? permission
    : 'unknown';
  return t(`settings:release.osNotifications.permissions.${knownPermission}`);
}
