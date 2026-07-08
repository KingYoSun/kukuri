import { beforeEach, describe, expect, test } from 'vitest';

import {
  buildSafeDiagnosticReport,
  classifyUpdateError,
  DEFAULT_OS_NOTIFICATION_SETTINGS,
  loadOsNotificationSettings,
  saveOsNotificationSettings,
  type OsNotificationSettings,
} from './releaseReadiness';

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

  test('update errors are classified for user-facing guidance', () => {
    expect(classifyUpdateError('Could not fetch a valid release JSON from the remote')).toBe(
      'manifest'
    );
    expect(classifyUpdateError('request timed out while contacting updater endpoint')).toBe(
      'network'
    );
    expect(classifyUpdateError('signature verification failed')).toBe('signature');
    expect(classifyUpdateError('failed to install update bundle')).toBe('install');
    expect(classifyUpdateError('unexpected updater failure')).toBe('unknown');
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
});
