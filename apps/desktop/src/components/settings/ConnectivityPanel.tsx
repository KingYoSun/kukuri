import { useTranslation } from 'react-i18next';

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

type ConnectivityPanelProps = {
  view: ConnectivityPanelView;
  onPeerTicketInputChange: (value: string) => void;
  onImportPeer: () => void;
};

export function ConnectivityPanel({
  view,
  onPeerTicketInputChange,
  onImportPeer,
}: ConnectivityPanelProps) {
  const { t } = useTranslation(['common', 'settings']);

  return (
    <div className='space-y-4'>
      <Card className='space-y-4'>
        <CardHeader className='items-start justify-between gap-3 md:flex'>
          <div>
            <h3>{t('settings:connectivity.title')}</h3>
            <small>{view.summaryLabel}</small>
          </div>
          <StatusBadge label={view.summaryLabel} tone={view.status === 'error' ? 'destructive' : 'accent'} />
        </CardHeader>

        {view.status === 'loading' ? <Notice>{t('settings:connectivity.loading')}</Notice> : null}
        {view.panelError ? <Notice tone='destructive'>{view.panelError}</Notice> : null}

        <SettingsMetricGrid items={view.metrics} />
        <SettingsDiagnosticList items={view.diagnostics} columns={2} />
      </Card>

      <Card className='space-y-4'>
        <CardHeader>
          <h3>{t('settings:connectivity.peerTickets')}</h3>
          <small>{t('settings:connectivity.manualConnectivity')}</small>
        </CardHeader>

        <label className='flex flex-col gap-3'>
          <span>{t('settings:connectivity.yourTicket')}</span>
          <Textarea
            readOnly
            value={view.localPeerTicket}
            className='min-h-[88px] resize-y font-mono text-[0.8rem]'
          />
        </label>

        <label className='flex flex-col gap-3'>
          <span>{t('settings:connectivity.peerTicket')}</span>
          <Input
            value={view.peerTicketInput}
            onChange={(event) => onPeerTicketInputChange(event.target.value)}
            placeholder={t('settings:connectivity.peerTicketPlaceholder')}
          />
        </label>

        <SettingsActionRow>
          <Button variant='secondary' onClick={onImportPeer}>
            {t('common:actions.importPeer')}
          </Button>
        </SettingsActionRow>
      </Card>

      <Card className='space-y-4'>
        <CardHeader>
          <h3>{t('settings:connectivity.topicConnectivity')}</h3>
          <small>{t('settings:connectivity.tracked', { count: view.topics.length })}</small>
        </CardHeader>

        {view.topics.length === 0 ? <Notice>{t('settings:connectivity.noTopicDiagnostics')}</Notice> : null}

        <div className='space-y-3'>
          {view.topics.map((topic) => (
            <section
              key={topic.topic}
              className='rounded-[20px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4 shadow-[0_12px_32px_rgba(2,7,15,0.1)]'
            >
              <div className='flex flex-wrap items-start justify-between gap-3'>
                <div className='min-w-0'>
                  <h4 className='break-all text-base font-semibold text-foreground'>{topic.topic}</h4>
                  <p className='mt-2 text-sm text-[var(--muted-foreground)]'>{topic.summary}</p>
                </div>
                <StatusBadge
                  label={t('settings:connectivity.lastReceivedBadge', {
                    value: topic.lastReceivedLabel,
                  })}
                />
              </div>

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
                      tone: topic.missingPeerCount > 0 ? 'warning' : 'default',
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
            </section>
          ))}
        </div>
      </Card>
    </div>
  );
}
