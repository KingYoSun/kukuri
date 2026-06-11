import { beforeEach, describe, expect, test } from 'vitest';

import type { NotificationView } from '@/lib/api';
import {
  buildSafeDiagnosticReport,
  DEFAULT_OS_NOTIFICATION_SETTINGS,
  loadOsNotificationSettings,
  notificationBody,
  saveOsNotificationSettings,
  shouldSendOsNotification,
  type OsNotificationSettings,
} from './releaseReadiness';

function notification(overrides: Partial<NotificationView> = {}): NotificationView {
  return {
    notification_id: 'notification-1',
    kind: 'direct_message',
    actor_pubkey: 'remote-author',
    actor_name: null,
    actor_display_name: null,
    actor_picture: null,
    actor_picture_asset: null,
    source_envelope_id: null,
    source_replica_id: null,
    topic_id: 'kukuri:topic:demo',
    channel_id: null,
    object_id: null,
    thread_root_object_id: null,
    dm_id: 'dm-1',
    message_id: 'message-1',
    preview_text: 'private body',
    created_at: 1,
    received_at: 1,
    read_at: null,
    ...overrides,
  };
}

describe('release readiness helpers', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  test('diagnostic report excludes secret-bearing fields and includes release state', () => {
    const report = buildSafeDiagnosticReport({
      appVersion: '0.1.0',
      updateState: {
        status: 'failed',
        currentVersion: '0.1.0',
        availableVersion: '0.1.1',
        lastError: 'manifest fetch failed',
      },
      osNotificationPermission: 'granted',
      osNotificationSettings: {
        ...DEFAULT_OS_NOTIFICATION_SETTINGS,
        enabled: true,
      },
      userAgent: 'test-agent',
      platform: 'Win32',
      syncConnected: true,
      deliveryState: 'Live',
      discoveryMode: 'seeded_dht',
      activePath: 'relay_supported_p2p',
      peerCount: 2,
      subscribedTopicCount: 4,
      unreadNotificationCount: 1,
      communityNodeStatuses: [
        {
          base_url: 'https://api.kukuri.app',
          session_phase: 'ready',
          retry_after: null,
          restart_required: false,
          last_error: null,
        },
      ],
      lastSyncError: null,
      lastDiscoveryError: 'transient discovery error',
    });

    expect(report).toContain('release_channel: preview');
    expect(report).toContain('update_status: failed');
    expect(report).toContain('base_url: https://api.kukuri.app');
    expect(report).toContain('last_discovery_error: transient discovery error');
    expect(report).not.toContain('secret-key');
    expect(report).not.toContain('auth-token');
    expect(report).not.toContain('private body');
    expect(report).toContain('secret keys, auth tokens, private channel capability secrets');
  });

  test('OS notification settings persist independently from local inbox state', () => {
    const settings: OsNotificationSettings = {
      ...DEFAULT_OS_NOTIFICATION_SETTINGS,
      enabled: true,
      previewBody: true,
    };

    saveOsNotificationSettings(settings);

    expect(loadOsNotificationSettings()).toEqual(settings);
  });

  test('OS notification filtering respects local author, read state, and kind settings', () => {
    const settings: OsNotificationSettings = {
      ...DEFAULT_OS_NOTIFICATION_SETTINGS,
      enabled: true,
      directMessages: true,
      mentionsAndReplies: false,
      followsAndReposts: false,
    };

    expect(shouldSendOsNotification(notification(), settings, 'local-author')).toBe(true);
    expect(
      shouldSendOsNotification(
        notification({ actor_pubkey: 'local-author' }),
        settings,
        'local-author'
      )
    ).toBe(false);
    expect(shouldSendOsNotification(notification({ read_at: 2 }), settings, 'local-author')).toBe(
      false
    );
    expect(
      shouldSendOsNotification(notification({ kind: 'mention' }), settings, 'local-author')
    ).toBe(false);
  });

  test('private direct message bodies are hidden unless preview body is enabled', () => {
    expect(
      notificationBody(notification(), {
        ...DEFAULT_OS_NOTIFICATION_SETTINGS,
        previewBody: false,
      })
    ).toBe('Open kukuri to read this message.');
    expect(
      notificationBody(notification(), {
        ...DEFAULT_OS_NOTIFICATION_SETTINGS,
        previewBody: true,
      })
    ).toBe('private body');
  });
});
