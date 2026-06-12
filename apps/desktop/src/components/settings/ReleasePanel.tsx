import { useCallback, useEffect, useMemo, useState } from 'react';
import { BookOpen, Download, ExternalLink, FileText, RefreshCw, ShieldCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import packageJson from '../../../package.json';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { copyTextToClipboard } from '@/lib/utils';
import {
  buildSafeDiagnosticReport,
  classifyUpdateError,
  DEFAULT_OS_NOTIFICATION_SETTINGS,
  isTauriRuntime,
  loadOsNotificationSettings,
  RELEASE_CHANNEL,
  RELEASE_FEEDBACK_URL,
  RELEASE_MANIFEST_NAME,
  RELEASE_RUNBOOK_URL,
  saveOsNotificationSettings,
  THIRD_PARTY_NOTICES_URL,
  type OsNotificationSettings,
  type UpdateState,
} from '@/lib/releaseReadiness';
import { useDesktopShellStore } from '@/shell/store';

import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';

const INITIAL_UPDATE_STATE: UpdateState = {
  status: 'idle',
  currentVersion: packageJson.version,
  availableVersion: null,
  downloadedBytes: 0,
  contentLength: null,
  lastError: null,
};

function formatUpdateStatus(status: UpdateState['status']): string {
  return status.replaceAll('_', ' ');
}

function updateErrorTranslationKey(errorMessage?: string | null): string {
  return `settings:release.update.errors.${classifyUpdateError(errorMessage)}`;
}

function updateStateFromError(currentVersion: string, error: unknown): UpdateState {
  return {
    status: 'failed',
    currentVersion,
    availableVersion: null,
    lastError: error instanceof Error ? error.message : String(error),
  };
}

type PendingUpdate = {
  version: string;
  downloadAndInstall: (onEvent?: (event: unknown) => void) => Promise<void>;
};

export function ReleasePanel() {
  const { t } = useTranslation(['common', 'settings']);
  const syncStatus = useDesktopShellStore((state) => state.syncStatus);
  const notificationStatus = useDesktopShellStore((state) => state.notificationStatus);
  const communityNodeStatuses = useDesktopShellStore((state) => state.communityNodeStatuses);
  const [updateState, setUpdateState] = useState<UpdateState>(INITIAL_UPDATE_STATE);
  const [pendingUpdate, setPendingUpdate] = useState<PendingUpdate | null>(null);
  const [diagnosticReport, setDiagnosticReport] = useState('');
  const [diagnosticMessage, setDiagnosticMessage] = useState<string | null>(null);
  const [osNotificationSettings, setOsNotificationSettings] =
    useState<OsNotificationSettings>(DEFAULT_OS_NOTIFICATION_SETTINGS);
  const [osNotificationPermission, setOsNotificationPermission] = useState('unknown');

  useEffect(() => {
    setOsNotificationSettings(loadOsNotificationSettings());
    if (!isTauriRuntime()) {
      return;
    }
    let cancelled = false;
    void import('@tauri-apps/plugin-notification').then(async (plugin) => {
      const granted = await plugin.isPermissionGranted();
      if (!cancelled) {
        setOsNotificationPermission(granted ? 'granted' : 'prompt');
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

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
    const plugin = await import('@tauri-apps/plugin-notification');
    const permission = await plugin.requestPermission();
    setOsNotificationPermission(permission);
    if (permission === 'granted') {
      updateOsNotificationSetting({ enabled: true });
    }
  }, [updateOsNotificationSetting]);

  const checkForUpdate = useCallback(async () => {
    setUpdateState((current) => ({
      ...current,
      status: 'checking',
      lastError: null,
    }));
    try {
      const [{ getVersion }, updater] = await Promise.all([
        import('@tauri-apps/api/app'),
        import('@tauri-apps/plugin-updater'),
      ]);
      const currentVersion = isTauriRuntime() ? await getVersion() : packageJson.version;
      const update = isTauriRuntime() ? await updater.check() : null;
      if (!update) {
        setPendingUpdate(null);
        setUpdateState({
          status: 'up_to_date',
          currentVersion,
          availableVersion: null,
          lastError: null,
        });
        return;
      }
      setPendingUpdate(update);
      setUpdateState({
        status: 'available',
        currentVersion,
        availableVersion: update.version,
        lastError: null,
      });
    } catch (error) {
      setUpdateState((current) => updateStateFromError(current.currentVersion, error));
    }
  }, []);

  const installUpdate = useCallback(async () => {
    if (!pendingUpdate) {
      await checkForUpdate();
      return;
    }
    setUpdateState((current) => ({
      ...current,
      status: 'downloading',
      downloadedBytes: 0,
      contentLength: null,
      lastError: null,
    }));
    try {
      await pendingUpdate.downloadAndInstall((event) => {
        if (!event || typeof event !== 'object' || !('event' in event)) {
          return;
        }
        const downloadEvent = event as {
          event: string;
          data?: { chunkLength?: number; contentLength?: number };
        };
        setUpdateState((current) => {
          if (downloadEvent.event === 'Started') {
            return {
              ...current,
              contentLength: downloadEvent.data?.contentLength ?? null,
              downloadedBytes: 0,
            };
          }
          if (downloadEvent.event === 'Progress') {
            return {
              ...current,
              downloadedBytes:
                (current.downloadedBytes ?? 0) + (downloadEvent.data?.chunkLength ?? 0),
            };
          }
          return current;
        });
      });
      setUpdateState((current) => ({
        ...current,
        status: 'ready_to_restart',
        lastError: null,
      }));
    } catch (error) {
      setUpdateState((current) => updateStateFromError(current.currentVersion, error));
    }
  }, [checkForUpdate, pendingUpdate]);

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
      value: formatUpdateStatus(updateState.status),
      tone: updateState.status === 'failed' ? ('danger' as const) : ('default' as const),
    },
  ];
  const updateErrorMessage = updateState.lastError
    ? t(updateErrorTranslationKey(updateState.lastError))
    : null;

  const securityDiagnostics = [
    {
      label: t('settings:release.security.csp'),
      value: t('settings:release.security.cspValue'),
    },
    {
      label: t('settings:release.security.updaterSignature'),
      value: t('settings:release.security.updaterSignatureValue'),
    },
    {
      label: t('settings:release.security.codeSigning'),
      value: t('settings:release.security.codeSigningValue'),
    },
  ];

  const dataSafetyDiagnostics = [
    {
      label: t('settings:release.dataSafety.identity'),
      value: t('settings:release.dataSafety.identityValue'),
    },
    {
      label: t('settings:release.dataSafety.localData'),
      value: t('settings:release.dataSafety.localDataValue'),
    },
    {
      label: t('settings:release.dataSafety.backup'),
      value: t('settings:release.dataSafety.backupValue'),
    },
    {
      label: t('settings:release.dataSafety.reset'),
      value: t('settings:release.dataSafety.resetValue'),
    },
  ];

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
        <SettingsDiagnosticList items={updateDiagnostics} columns={2} />
        {updateState.lastError ? (
          <Notice tone='destructive'>
            <div className='space-y-1'>
              <p>{updateErrorMessage}</p>
              <small className='font-mono'>{updateState.lastError}</small>
            </div>
          </Notice>
        ) : null}
        {updateState.availableVersion ? (
          <Notice tone='accent'>
            {t('settings:release.update.available', { version: updateState.availableVersion })}
          </Notice>
        ) : null}
        <SettingsActionRow>
          <Button
            variant='secondary'
            type='button'
            disabled={updateState.status === 'checking' || updateState.status === 'downloading'}
            onClick={() => void checkForUpdate()}
          >
            <RefreshCw className='size-4' aria-hidden='true' />
            {t('settings:release.update.check')}
          </Button>
          <Button
            variant='secondary'
            type='button'
            disabled={!pendingUpdate || updateState.status === 'downloading'}
            onClick={() => void installUpdate()}
          >
            <Download className='size-4' aria-hidden='true' />
            {t('settings:release.update.install')}
          </Button>
        </SettingsActionRow>
      </section>

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

      <section className='min-w-0 space-y-3'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('settings:release.osNotifications.title')}
        </h4>
        <Notice>{t('settings:release.osNotifications.permission', { osNotificationPermission })}</Notice>
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

      <section className='min-w-0 space-y-3'>
        <h4 className='flex items-center gap-2 text-base font-semibold text-foreground'>
          <ShieldCheck className='size-4' aria-hidden='true' />
          {t('settings:release.security.title')}
        </h4>
        <SettingsDiagnosticList items={securityDiagnostics} columns={2} />
        <Notice>{t('settings:release.privacy')}</Notice>
      </section>

      <section className='min-w-0 space-y-3'>
        <h4 className='flex items-center gap-2 text-base font-semibold text-foreground'>
          <BookOpen className='size-4' aria-hidden='true' />
          {t('settings:release.dataSafety.title')}
        </h4>
        <SettingsDiagnosticList items={dataSafetyDiagnostics} columns={2} />
        <SettingsActionRow>
          <Button
            variant='secondary'
            type='button'
            onClick={() => {
              window.open(RELEASE_RUNBOOK_URL, '_blank', 'noopener,noreferrer');
            }}
          >
            <ExternalLink className='size-4' aria-hidden='true' />
            {t('settings:release.dataSafety.releaseRunbook')}
          </Button>
          <Button
            variant='secondary'
            type='button'
            onClick={() => {
              window.open(THIRD_PARTY_NOTICES_URL, '_blank', 'noopener,noreferrer');
            }}
          >
            <ExternalLink className='size-4' aria-hidden='true' />
            {t('settings:release.dataSafety.thirdPartyNotices')}
          </Button>
        </SettingsActionRow>
      </section>
    </Card>
  );
}
