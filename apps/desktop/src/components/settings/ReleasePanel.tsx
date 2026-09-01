import { useCallback, useEffect, useMemo, useState } from 'react';
import { Download, ExternalLink, FileText, Power, RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import {
  getOsNotificationPermission,
  requestOsNotificationPermission as requestOsNotificationPermissionCommand,
} from '@/lib/api/osNotificationPermission';
import { copyTextToClipboard } from '@/lib/utils';
import {
  buildSafeDiagnosticReport,
  classifyUpdateError,
  DEFAULT_OS_NOTIFICATION_SETTINGS,
  isTauriRuntime,
  loadOsNotificationSettings,
  RELEASE_CHANNEL,
  RELEASE_FEEDBACK_URL,
  RELEASE_LATEST_URL,
  RELEASE_MANIFEST_NAME,
  RELEASE_QUICKSTART_URL,
  RELEASE_RUNBOOK_URL,
  saveOsNotificationSettings,
  THIRD_PARTY_NOTICES_URL,
  type OsNotificationSettings,
} from '@/lib/releaseReadiness';
import { buildCommunityNodeDisclosures } from '@/lib/communityNodeDisclosures';
import { useAppUpdateStore } from '@/shell/useAppUpdateStore';
import { useDesktopShellStore } from '@/shell/store';

import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { formatOsNotificationPermission, formatUpdateStatus } from './releasePanelCopy';

function updateErrorTranslationKey(errorMessage?: string | null): string {
  return `settings:release.update.errors.${classifyUpdateError(errorMessage)}`;
}

type ReleasePanelProps = {
  showDiagnostics?: boolean;
};

export function ReleasePanel({ showDiagnostics = true }: ReleasePanelProps) {
  const { t } = useTranslation(['common', 'settings']);
  const syncStatus = useDesktopShellStore((state) => state.syncStatus);
  const notificationStatus = useDesktopShellStore((state) => state.notificationStatus);
  const communityNodeStatuses = useDesktopShellStore((state) => state.communityNodeStatuses);
  const communityNodeConfig = useDesktopShellStore((state) => state.communityNodeConfig);
  const communityNodeManifests = useDesktopShellStore((state) => state.communityNodeManifests);
  const updateState = useAppUpdateStore((state) => state.updateState);
  const pendingUpdate = useAppUpdateStore((state) => state.pendingUpdate);
  const checkForUpdate = useAppUpdateStore((state) => state.checkForUpdate);
  const downloadUpdate = useAppUpdateStore((state) => state.downloadUpdate);
  const restartAndInstall = useAppUpdateStore((state) => state.restartAndInstall);
  const [diagnosticReport, setDiagnosticReport] = useState('');
  const [diagnosticMessage, setDiagnosticMessage] = useState<string | null>(null);
  const [restartPromptDismissed, setRestartPromptDismissed] = useState(false);
  const [osNotificationSettings, setOsNotificationSettings] =
    useState<OsNotificationSettings>(DEFAULT_OS_NOTIFICATION_SETTINGS);
  const [osNotificationPermission, setOsNotificationPermission] = useState('unknown');

  useEffect(() => {
    setOsNotificationSettings(loadOsNotificationSettings());
    if (!isTauriRuntime()) {
      return;
    }
    let cancelled = false;
    // Query the Tauri backend directly instead of the WebView Web Notification
    // API, whose permission state is volatile and unreliable on Windows (#313).
    void getOsNotificationPermission()
      .then((permission) => {
        if (!cancelled) {
          setOsNotificationPermission(permission.toLowerCase());
        }
      })
      .catch(() => {
        if (!cancelled) {
          setOsNotificationPermission('prompt');
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (updateState.status !== 'ready_to_restart') {
      setRestartPromptDismissed(false);
    }
  }, [updateState.status]);

  const updateOsNotificationSetting = useCallback(
    (patch: Partial<OsNotificationSettings>) => {
      const next = {
        ...osNotificationSettings,
        ...patch,
      };
      setOsNotificationSettings(next);
      saveOsNotificationSettings(next);
    },
    [osNotificationSettings]
  );

  const requestOsNotificationPermission = useCallback(async () => {
    if (!isTauriRuntime()) {
      setOsNotificationPermission('unavailable');
      return;
    }
    const permission = await requestOsNotificationPermissionCommand();
    const normalized = permission.toLowerCase();
    setOsNotificationPermission(normalized);
    if (normalized === 'granted') {
      updateOsNotificationSetting({ enabled: true });
    }
  }, [updateOsNotificationSetting]);

  const diagnosticReportText = useMemo(
    () =>
      buildSafeDiagnosticReport({
        appVersion: updateState.currentVersion,
        updateState,
        osNotificationPermission,
        osNotificationSettings,
        userAgent: typeof navigator === 'undefined' ? 'unknown' : navigator.userAgent,
        platform: typeof navigator === 'undefined' ? 'unknown' : navigator.platform,
        syncConnected: syncStatus.connected,
        deliveryState: syncStatus.delivery_state,
        discoveryMode: syncStatus.discovery.mode,
        activePath: syncStatus.active_path,
        peerCount: syncStatus.peer_count,
        subscribedTopicCount: syncStatus.subscribed_topics.length,
        unreadNotificationCount: notificationStatus.unread_count,
        communityNodeStatuses,
        lastSyncError: syncStatus.last_error,
        lastDiscoveryError: syncStatus.discovery.last_discovery_error,
      }),
    [
      communityNodeStatuses,
      notificationStatus.unread_count,
      osNotificationPermission,
      osNotificationSettings,
      syncStatus.active_path,
      syncStatus.connected,
      syncStatus.delivery_state,
      syncStatus.discovery.last_discovery_error,
      syncStatus.discovery.mode,
      syncStatus.last_error,
      syncStatus.peer_count,
      syncStatus.subscribed_topics.length,
      updateState,
    ]
  );

  useEffect(() => {
    if (diagnosticReport) {
      setDiagnosticReport(diagnosticReportText);
    }
  }, [diagnosticReport, diagnosticReportText]);

  const copyDiagnosticReport = useCallback(async () => {
    const copied = await copyTextToClipboard(diagnosticReportText);
    setDiagnosticReport(diagnosticReportText);
    setDiagnosticMessage(
      copied
        ? t('settings:release.diagnostics.copied')
        : t('settings:release.diagnostics.copyUnavailable')
    );
  }, [diagnosticReportText, t]);

  const exportDiagnosticReport = useCallback(() => {
    const blob = new Blob([diagnosticReportText], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'kukuri-diagnostics.txt';
    link.click();
    URL.revokeObjectURL(url);
    setDiagnosticReport(diagnosticReportText);
    setDiagnosticMessage(t('settings:release.diagnostics.exported'));
  }, [diagnosticReportText, t]);

  const updateDiagnostics = [
    {
      label: t('settings:release.update.version'),
      value: updateState.currentVersion,
      monospace: true,
    },
    {
      label: t('settings:release.update.channel'),
      value: RELEASE_CHANNEL,
    },
    {
      label: t('settings:release.update.manifest'),
      value: RELEASE_MANIFEST_NAME,
      monospace: true,
    },
    {
      label: t('settings:release.update.status'),
      value: formatUpdateStatus(updateState.status, t),
      tone: updateState.status === 'failed' ? ('danger' as const) : ('default' as const),
    },
  ];
  const updateErrorMessage = updateState.lastError
    ? t(updateErrorTranslationKey(updateState.lastError))
    : null;
  const updateBusy = updateState.status === 'checking' || updateState.status === 'downloading';
  const updateReadyToRestart = updateState.status === 'ready_to_restart';
  const communityNodeDisclosures = useMemo(
    () => buildCommunityNodeDisclosures(communityNodeConfig, communityNodeManifests),
    [communityNodeConfig, communityNodeManifests]
  );

  return (
    <Card className='min-w-0 space-y-5'>
      <CardHeader>
        <h3>{t('settings:release.title')}</h3>
        <small>{t('settings:release.summary')}</small>
      </CardHeader>

      <section className='min-w-0 space-y-3'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('settings:release.update.title')}
        </h4>
        {showDiagnostics ? <SettingsDiagnosticList items={updateDiagnostics} columns={2} /> : null}
        {updateState.lastError ? (
          <Notice tone='destructive'>
            <div className='space-y-1'>
              <p>{updateErrorMessage}</p>
              {showDiagnostics ? (
                <small className='font-mono'>{updateState.lastError}</small>
              ) : null}
            </div>
          </Notice>
        ) : null}
        {updateReadyToRestart && !restartPromptDismissed ? (
          <Notice tone='warning'>
            <div className='space-y-3'>
              <div className='space-y-1'>
                <p className='font-semibold'>{t('settings:release.update.ready')}</p>
                <p>{t('settings:release.update.readyDescription')}</p>
                <small>{t('settings:release.update.restartWarning')}</small>
              </div>
              <SettingsActionRow>
                <Button
                  type='button'
                  onClick={() => void restartAndInstall()}
                  disabled={!pendingUpdate}
                >
                  <Power className='size-4' aria-hidden='true' />
                  {t('settings:release.update.restartNow')}
                </Button>
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() => setRestartPromptDismissed(true)}
                >
                  {t('settings:release.update.later')}
                </Button>
              </SettingsActionRow>
            </div>
          </Notice>
        ) : null}
        {updateState.availableVersion && !updateReadyToRestart ? (
          <Notice tone='accent'>
            {t('settings:release.update.available', { version: updateState.availableVersion })}
          </Notice>
        ) : null}
        <SettingsActionRow>
          <Button
            variant='secondary'
            type='button'
            disabled={updateBusy}
            onClick={() => void checkForUpdate()}
          >
            <RefreshCw className='size-4' aria-hidden='true' />
            {t('settings:release.update.check')}
          </Button>
          {updateReadyToRestart ? (
            <Button
              variant='secondary'
              type='button'
              disabled={!pendingUpdate}
              onClick={() => void restartAndInstall()}
            >
              <Power className='size-4' aria-hidden='true' />
              {t('settings:release.update.restartNow')}
            </Button>
          ) : (
            <Button
              variant='secondary'
              type='button'
              disabled={!pendingUpdate || updateBusy}
              onClick={() => void downloadUpdate()}
            >
              <Download className='size-4' aria-hidden='true' />
              {t('settings:release.update.install')}
            </Button>
          )}
        </SettingsActionRow>
      </section>

      <section className='min-w-0 space-y-3'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('settings:release.resources.title')}
        </h4>
        <p className='text-sm text-[var(--muted-foreground-soft)]'>
          {t('settings:release.resources.summary')}
        </p>
        <SettingsActionRow>
          {[
            [t('settings:release.resources.latestRelease'), RELEASE_LATEST_URL],
            [t('settings:release.resources.quickstart'), RELEASE_QUICKSTART_URL],
            [t('settings:release.resources.releaseRunbook'), RELEASE_RUNBOOK_URL],
            [t('settings:release.resources.thirdPartyNotices'), THIRD_PARTY_NOTICES_URL],
          ].map(([label, href]) => (
            <Button key={href} asChild variant='secondary'>
              <a href={href} target='_blank' rel='noreferrer'>
                {label}
                <ExternalLink className='size-4' aria-hidden='true' />
              </a>
            </Button>
          ))}
        </SettingsActionRow>
        <p className='text-sm font-medium text-foreground'>
          {t('settings:release.resources.communityNodeDisclosures')}
        </p>
        {communityNodeDisclosures.length === 0 ? (
          <p className='text-sm text-[var(--muted-foreground-soft)]'>
            {t('settings:release.resources.noCommunityNodes')}
          </p>
        ) : (
          <div className='space-y-3'>
            {communityNodeDisclosures.map((disclosure) => (
              <div
                key={disclosure.baseUrl}
                className='space-y-2 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-3'
              >
                <div className='min-w-0'>
                  <p className='font-medium text-foreground'>
                    {disclosure.nodeName ?? disclosure.baseUrl}
                  </p>
                  {disclosure.nodeName ? (
                    <small className='break-all font-mono'>{disclosure.baseUrl}</small>
                  ) : null}
                </div>
                {disclosure.manifestAvailable && disclosure.links.length > 0 ? (
                  <SettingsActionRow>
                    {disclosure.links.map(({ key, href }) => (
                      <Button key={`${key}:${href}`} asChild variant='secondary' size='sm'>
                        <a href={href} target='_blank' rel='noreferrer'>
                          {t(`settings:release.resources.${key}`)}
                          <ExternalLink className='size-4' aria-hidden='true' />
                        </a>
                      </Button>
                    ))}
                  </SettingsActionRow>
                ) : (
                  <small>{t('settings:release.resources.manifestUnavailable')}</small>
                )}
              </div>
            ))}
          </div>
        )}
      </section>

      {showDiagnostics ? (
      <section className='min-w-0 space-y-3'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('settings:release.diagnostics.title')}
        </h4>
        <SettingsActionRow>
          <Button variant='secondary' type='button' onClick={() => void copyDiagnosticReport()}>
            <FileText className='size-4' aria-hidden='true' />
            {t('settings:release.diagnostics.copy')}
          </Button>
          <Button variant='secondary' type='button' onClick={exportDiagnosticReport}>
            <Download className='size-4' aria-hidden='true' />
            {t('settings:release.diagnostics.export')}
          </Button>
          <Button
            variant='secondary'
            type='button'
            onClick={() => {
              window.open(RELEASE_FEEDBACK_URL, '_blank', 'noopener,noreferrer');
            }}
          >
            {t('settings:release.diagnostics.feedback')}
          </Button>
        </SettingsActionRow>
        {diagnosticMessage ? <Notice tone='accent'>{diagnosticMessage}</Notice> : null}
        {diagnosticReport ? (
          <textarea
            className='min-h-44 w-full resize-y rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-3 font-mono text-xs text-[var(--muted-foreground-soft)]'
            readOnly
            value={diagnosticReport}
            aria-label={t('settings:release.diagnostics.previewLabel')}
          />
        ) : null}
      </section>
      ) : null}

      <section className='min-w-0 space-y-3'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('settings:release.osNotifications.title')}
        </h4>
        <Notice>
          {t('settings:release.osNotifications.permission', {
            permission: formatOsNotificationPermission(osNotificationPermission, t),
          })}
        </Notice>
        <div className='grid gap-3 sm:grid-cols-2'>
          {[
            ['enabled', t('settings:release.osNotifications.enabled')],
            ['directMessages', t('settings:release.osNotifications.directMessages')],
            ['mentionsAndReplies', t('settings:release.osNotifications.mentionsAndReplies')],
            ['followsAndReposts', t('settings:release.osNotifications.followsAndReposts')],
            ['quietMode', t('settings:release.osNotifications.quietMode')],
            ['previewBody', t('settings:release.osNotifications.previewBody')],
          ].map(([key, label]) => (
            <label
              key={key}
              className='flex min-w-0 items-center gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] px-4 py-3 text-sm text-foreground'
            >
              <input
                type='checkbox'
                checked={Boolean(osNotificationSettings[key as keyof OsNotificationSettings])}
                onChange={(event) =>
                  updateOsNotificationSetting({
                    [key]: event.currentTarget.checked,
                  } as Partial<OsNotificationSettings>)
                }
              />
              <span>{label}</span>
            </label>
          ))}
        </div>
        <SettingsActionRow>
          <Button
            variant='secondary'
            type='button'
            onClick={() => void requestOsNotificationPermission()}
          >
            {t('settings:release.osNotifications.requestPermission')}
          </Button>
        </SettingsActionRow>
      </section>
    </Card>
  );
}
