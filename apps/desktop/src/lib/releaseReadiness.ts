import type { NotificationView } from '@/lib/api';

export const RELEASE_CHANNEL = 'preview';
export const RELEASE_MANIFEST_NAME = 'latest-preview.json';
export const RELEASE_FEEDBACK_URL =
  'https://github.com/KingYoSun/kukuri/issues/new?template=preview-feedback.md';
export const RELEASE_RUNBOOK_URL =
  'https://github.com/KingYoSun/kukuri/blob/main/docs/runbooks/release.md';
export const THIRD_PARTY_NOTICES_URL =
  'https://github.com/KingYoSun/kukuri/blob/main/docs/THIRD_PARTY_NOTICES.md';

export const OS_NOTIFICATION_SETTINGS_STORAGE_KEY = 'kukuri:os-notification-settings:v1';
const OS_NOTIFICATION_SEEN_STORAGE_KEY = 'kukuri:os-notification-seen:v1';
const MAX_SEEN_NOTIFICATION_IDS = 128;

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'up_to_date'
  | 'available'
  | 'downloading'
  | 'ready_to_restart'
  | 'failed';

export type UpdateState = {
  status: UpdateStatus;
  currentVersion: string;
  availableVersion?: string | null;
  downloadedBytes?: number;
  contentLength?: number | null;
  lastError?: string | null;
};

export type OsNotificationSettings = {
  enabled: boolean;
  directMessages: boolean;
  mentionsAndReplies: boolean;
  followsAndReposts: boolean;
  quietMode: boolean;
  previewBody: boolean;
};

export const DEFAULT_OS_NOTIFICATION_SETTINGS: OsNotificationSettings = {
  enabled: false,
  directMessages: true,
  mentionsAndReplies: true,
  followsAndReposts: false,
  quietMode: false,
  previewBody: false,
};

export function loadOsNotificationSettings(): OsNotificationSettings {
  if (typeof window === 'undefined') {
    return DEFAULT_OS_NOTIFICATION_SETTINGS;
  }
  const rawValue = window.localStorage.getItem(OS_NOTIFICATION_SETTINGS_STORAGE_KEY);
  if (!rawValue) {
    return DEFAULT_OS_NOTIFICATION_SETTINGS;
  }
  try {
    const parsed = JSON.parse(rawValue) as Partial<OsNotificationSettings>;
    return {
      ...DEFAULT_OS_NOTIFICATION_SETTINGS,
      ...parsed,
    };
  } catch {
    return DEFAULT_OS_NOTIFICATION_SETTINGS;
  }
}

export function saveOsNotificationSettings(settings: OsNotificationSettings): void {
  if (typeof window === 'undefined') {
    return;
  }
  window.localStorage.setItem(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  window.dispatchEvent(new Event(OS_NOTIFICATION_SETTINGS_STORAGE_KEY));
}

export function shouldSendOsNotification(
  notification: NotificationView,
  settings: OsNotificationSettings,
  localAuthorPubkey: string
): boolean {
  if (!settings.enabled || settings.quietMode || notification.read_at) {
    return false;
  }
  if (notification.actor_pubkey === localAuthorPubkey) {
    return false;
  }
  if (notification.kind === 'direct_message') {
    return settings.directMessages;
  }
  if (notification.kind === 'mention' || notification.kind === 'reply') {
    return settings.mentionsAndReplies;
  }
  if (
    notification.kind === 'followed' ||
    notification.kind === 'repost' ||
    notification.kind === 'quote_repost'
  ) {
    return settings.followsAndReposts;
  }
  return false;
}

export function notificationTitle(notification: NotificationView): string {
  switch (notification.kind) {
    case 'direct_message':
      return 'Direct message';
    case 'mention':
      return 'Mention';
    case 'reply':
      return 'Reply';
    case 'followed':
      return 'New follower';
    case 'quote_repost':
      return 'Quote repost';
    case 'repost':
      return 'Repost';
  }
}

export function notificationBody(
  notification: NotificationView,
  settings: OsNotificationSettings
): string | undefined {
  if (!settings.previewBody) {
    return notification.kind === 'direct_message'
      ? 'Open kukuri to read this message.'
      : 'Open kukuri to view this activity.';
  }
  return notification.preview_text ?? undefined;
}

export function nextOsNotificationId(notificationId: string): number {
  let hash = 0;
  for (let index = 0; index < notificationId.length; index += 1) {
    hash = (hash * 31 + notificationId.charCodeAt(index)) >>> 0;
  }
  return hash & 0x7fffffff;
}

export function readSeenOsNotificationIds(): Set<string> {
  if (typeof window === 'undefined') {
    return new Set();
  }
  try {
    const parsed = JSON.parse(
      window.localStorage.getItem(OS_NOTIFICATION_SEEN_STORAGE_KEY) ?? '[]'
    ) as string[];
    return new Set(parsed.filter((value) => typeof value === 'string'));
  } catch {
    return new Set();
  }
}

export function writeSeenOsNotificationIds(values: Set<string>): void {
  if (typeof window === 'undefined') {
    return;
  }
  const nextValues = Array.from(values).slice(-MAX_SEEN_NOTIFICATION_IDS);
  window.localStorage.setItem(OS_NOTIFICATION_SEEN_STORAGE_KEY, JSON.stringify(nextValues));
}

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function buildSafeDiagnosticReport(input: {
  appVersion: string;
  updateState: UpdateState;
  osNotificationPermission: string;
  osNotificationSettings: OsNotificationSettings;
  userAgent: string;
  platform: string;
  syncConnected: boolean;
  deliveryState: string;
  discoveryMode: string;
  activePath: string;
  peerCount: number;
  subscribedTopicCount: number;
  unreadNotificationCount: number;
  communityNodeStatuses: Array<{
    base_url: string;
    session_phase?: string | null;
    retry_after?: number | null;
    restart_required: boolean;
    last_error?: string | null;
  }>;
  lastSyncError?: string | null;
  lastDiscoveryError?: string | null;
}): string {
  const lines = [
    '# kukuri preview diagnostic report',
    '',
    `app_version: ${input.appVersion}`,
    `release_channel: ${RELEASE_CHANNEL}`,
    `platform: ${input.platform}`,
    `user_agent: ${input.userAgent}`,
    `sync_connected: ${input.syncConnected ? 'yes' : 'no'}`,
    `delivery_state: ${input.deliveryState}`,
    `discovery_mode: ${input.discoveryMode}`,
    `active_path: ${input.activePath}`,
    `peer_count: ${input.peerCount}`,
    `subscribed_topic_count: ${input.subscribedTopicCount}`,
    `unread_notification_count: ${input.unreadNotificationCount}`,
    `update_status: ${input.updateState.status}`,
    `update_current_version: ${input.updateState.currentVersion}`,
    `update_available_version: ${input.updateState.availableVersion ?? 'none'}`,
    `update_last_error: ${input.updateState.lastError ?? 'none'}`,
    `os_notification_permission: ${input.osNotificationPermission}`,
    `os_notifications_enabled: ${input.osNotificationSettings.enabled ? 'yes' : 'no'}`,
    `last_sync_error: ${input.lastSyncError ?? 'none'}`,
    `last_discovery_error: ${input.lastDiscoveryError ?? 'none'}`,
    '',
    'community_nodes:',
  ];

  if (input.communityNodeStatuses.length === 0) {
    lines.push('- none');
  } else {
    for (const status of input.communityNodeStatuses) {
      lines.push(
        [
          `- base_url: ${status.base_url}`,
          `session_phase: ${status.session_phase ?? 'unknown'}`,
          `retry_after: ${status.retry_after ?? 'none'}`,
          `restart_required: ${status.restart_required ? 'yes' : 'no'}`,
          `last_error: ${status.last_error ?? 'none'}`,
        ].join('; ')
      );
    }
  }

  lines.push(
    '',
    'redaction:',
    '- secret keys, auth tokens, private channel capability secrets, invite/share tokens, DM bodies, and local DB paths are not included.'
  );

  return lines.join('\n');
}
