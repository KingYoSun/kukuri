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
  return (
    <div className='space-y-4'>
      <Card className='space-y-4'>
        <CardHeader className='items-start justify-between gap-3 md:flex'>
          <div>
            <h3>Sync Status</h3>
            <small>{view.summaryLabel}</small>
          </div>
          <StatusBadge label={view.summaryLabel} tone={view.status === 'error' ? 'destructive' : 'accent'} />
        </CardHeader>

        {view.status === 'loading' ? <Notice>Loading connectivity diagnostics…</Notice> : null}
        {view.panelError ? <Notice tone='destructive'>{view.panelError}</Notice> : null}

        <SettingsMetricGrid items={view.metrics} />
        <SettingsDiagnosticList items={view.diagnostics} columns={2} />
      </Card>

      <Card className='space-y-4'>
        <CardHeader>
          <h3>Peer Tickets</h3>
          <small>manual connectivity</small>
        </CardHeader>

        <label className='flex flex-col gap-3'>
          <span>Your Ticket</span>
          <Textarea
            readOnly
            value={view.localPeerTicket}
            className='min-h-[88px] resize-y font-mono text-[0.8rem]'
          />
        </label>

        <label className='flex flex-col gap-3'>
          <span>Peer Ticket</span>
          <Input
            value={view.peerTicketInput}
            onChange={(event) => onPeerTicketInputChange(event.target.value)}
            placeholder='nodeid@127.0.0.1:7777'
          />
        </label>

        <SettingsActionRow>
          <Button variant='secondary' onClick={onImportPeer}>
            Import Peer
          </Button>
        </SettingsActionRow>
      </Card>

      <Card className='space-y-4'>
        <CardHeader>
          <h3>Topic Connectivity Detail</h3>
          <small>{view.topics.length} tracked</small>
        </CardHeader>

        {view.topics.length === 0 ? <Notice>No topic diagnostics yet.</Notice> : null}

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
                <StatusBadge label={`last received ${topic.lastReceivedLabel}`} />
              </div>

              <div className='mt-4'>
                <SettingsMetricGrid
                  items={[
                    { label: 'Expected', value: String(topic.expectedPeerCount) },
                    {
                      label: 'Missing',
                      value: String(topic.missingPeerCount),
                      tone: topic.missingPeerCount > 0 ? 'warning' : 'default',
                    },
                    { label: 'Last Received', value: topic.lastReceivedLabel },
                  ]}
                />
              </div>

              <div className='mt-4'>
                <SettingsDiagnosticList
                  items={[
                    { label: 'Status Detail', value: topic.statusDetail },
                    { label: 'Connected Peers', value: topic.connectedPeersLabel, monospace: true },
                    {
                      label: 'Relay-assisted Peers',
                      value: topic.relayAssistedPeersLabel,
                      monospace: true,
                    },
                    { label: 'Configured Peers', value: topic.configuredPeersLabel, monospace: true },
                    { label: 'Missing Peers', value: topic.missingPeersLabel, monospace: true },
                    {
                      label: 'Last Error',
                      value: topic.lastError ?? 'none',
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
