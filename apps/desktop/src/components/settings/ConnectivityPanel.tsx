import { useTranslation } from 'react-i18next';
import { ConnectivityGuidanceNotice, type DiagnosticActions } from './ConnectivityGuidanceNotice';

import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { SettingsMetricGrid } from './SettingsMetricGrid';
import { type ConnectivityPanelView } from './types';
import { topicDisplayName } from '@/lib/topicId';

type ConnectivityPanelProps = DiagnosticActions & {
  view: ConnectivityPanelView;
  onPeerTicketInputChange: (value: string) => void;
  onImportPeer: () => void;
  showDiagnostics?: boolean;
};

export function ConnectivityPanel({
  view,
  onRefreshDiagnostics,
  onOpenCommunityNode,
  onPeerTicketInputChange,
  onImportPeer,
  showDiagnostics = true,
}: ConnectivityPanelProps) {
  const { t } = useTranslation(['common', 'settings']);

  return (
    <div className='min-w-0 space-y-4'>
      <Card className='min-w-0 max-w-full space-y-4'>
        <CardHeader className='items-start justify-between gap-3 md:flex'>
          <div className='min-w-0'>
            <h3>{t('settings:connectivity.title')}</h3>
          </div>
          <StatusBadge label={view.summaryLabel} tone={view.guidance?.tone === 'warning' ? 'warning' : view.status === 'error' ? 'destructive' : 'accent'} />
        </CardHeader>

        {view.status === 'loading' ? <Notice>{t('settings:connectivity.loading')}</Notice> : null}

        <ConnectivityGuidanceNotice guidance={view.guidance} onRefreshDiagnostics={onRefreshDiagnostics} onOpenCommunityNode={onOpenCommunityNode} />
        {showDiagnostics && view.guidance?.loaded !== false ? (
          <>
            <SettingsMetricGrid items={view.metrics} />
            <p className='text-sm text-muted-foreground'>{t('settings:connectionGuidance.candidates')}</p>
            <details className='min-w-0'>
              <summary className='cursor-pointer py-2'>{t('settings:connectionGuidance.details')}</summary>
            <SettingsDiagnosticList items={view.diagnostics} columns={2} />
            </details>
          </>
        ) : null}
      </Card>

      {view.panelError ? <Notice tone='destructive'>
        <p>{t('settings:connectionGuidance.operationError')}</p>
        <details hidden={!showDiagnostics}>
          <summary className='cursor-pointer py-2'>{t('settings:connectionGuidance.operationDetails')}</summary>
          <p className='[overflow-wrap:anywhere]'>{view.panelError}</p>
        </details>
      </Notice> : null}

      <Card className='min-w-0 max-w-full space-y-4'>
        <CardHeader>
          <h3>{t('settings:connectivity.peerTickets')}</h3>
          <small>{t('settings:connectivity.manualConnectivity')}</small>
        </CardHeader>

        <label className='flex min-w-0 flex-col gap-3'>
          <span>{t('settings:connectivity.yourTicket')}</span>
          <Textarea
            readOnly
            value={view.localPeerTicket}
            className='min-h-[88px] min-w-0 max-w-full resize-y font-mono text-[0.8rem] [overflow-wrap:anywhere]'
          />
        </label>

        <label className='flex min-w-0 flex-col gap-3'>
          <span>{t('settings:connectivity.peerTicket')}</span>
          <Input
            value={view.peerTicketInput}
            onChange={(event) => onPeerTicketInputChange(event.target.value)}
            placeholder={t('settings:connectivity.peerTicketPlaceholder')}
            className='min-w-0 max-w-full'
          />
        </label>

        <SettingsActionRow>
          <Button variant='secondary' onClick={onImportPeer}>
            {t('common:actions.importPeer')}
          </Button>
        </SettingsActionRow>
      </Card>

      <Card className='min-w-0 max-w-full space-y-4'>
        <CardHeader>
          <h3>{t('settings:connectivity.topicConnectivity')}</h3>
          <small>{t('settings:connectivity.tracked', { count: view.topics.length })}</small>
        </CardHeader>

        {view.topics.length === 0 ? <Notice>{t('settings:connectivity.noTopicDiagnostics')}</Notice> : null}

        <div className='min-w-0 space-y-3'>
          {view.topics.map((topic) => (
            <section
              key={topic.topic}
              className='min-w-0 max-w-full rounded-[20px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4 shadow-[var(--shadow-dropdown)]'
            >
              <div className='flex flex-wrap items-start justify-between gap-3'>
                <div className='min-w-0'>
                  <h4 className='break-all text-base font-semibold text-foreground' title={topic.topic}>{topicDisplayName(topic.topic)}</h4>
                  <p className='mt-2 text-sm text-[var(--muted-foreground)]'>{topic.summary}</p>
                </div>
                {topic.expectedPeerCount !== null && topic.guidance?.loaded !== false ? <StatusBadge
                  label={t('settings:connectivity.lastReceivedBadge', {
                    value: topic.lastReceivedLabel,
                  })}
                /> : null}
              </div>

              <div className='mt-3'><ConnectivityGuidanceNotice guidance={topic.guidance} /></div>
              {showDiagnostics && topic.guidance?.loaded !== false && topic.expectedPeerCount !== null ? (
                <details className='min-w-0'>
                  <summary className='cursor-pointer py-2'>{t('settings:connectionGuidance.details')}</summary>
                  <div className='mt-4'>
                    <SettingsMetricGrid
                      items={[
                        {
                          label: t('settings:connectivity.metrics.expected'),
                          value: String(topic.expectedPeerCount),
                        },
                        {
                          label: t('settings:connectivity.metrics.missing'),
                          value: String(topic.missingPeerCount),
                          tone: (topic.missingPeerCount ?? 0) > 0 ? 'warning' : 'default',
                        },
                        {
                          label: t('settings:connectivity.metrics.lastReceived'),
                          value: topic.lastReceivedLabel,
                        },
                      ]}
                    />
                  </div>

                  <div className='mt-4'>
                    <SettingsDiagnosticList
                      items={[
                        {
                          label: t('settings:connectivity.diagnostics.statusDetail'),
                          value: topic.statusDetail,
                        },
                        {
                          label: t('settings:connectivity.diagnostics.connectedPeers'),
                          value: topic.connectedPeersLabel,
                          monospace: true,
                        },
                        {
                          label: t('settings:connectivity.diagnostics.relayAssistedPeers'),
                          value: topic.relayAssistedPeersLabel,
                          monospace: true,
                        },
                        {
                          label: t('settings:connectivity.diagnostics.configuredPeers'),
                          value: topic.configuredPeersLabel,
                          monospace: true,
                        },
                        {
                          label: t('settings:connectivity.diagnostics.missingPeers'),
                          value: topic.missingPeersLabel,
                          monospace: true,
                        },
                        {
                          label: t('settings:connectivity.diagnostics.lastError'),
                          value: topic.lastError ?? t('common:fallbacks.none'),
                          tone: topic.lastError ? 'danger' : 'default',
                        },
                      ]}
                      columns={2}
                    />
                  </div>
                </details>
              ) : null}
            </section>
          ))}
        </div>
      </Card>
    </div>
  );
}
