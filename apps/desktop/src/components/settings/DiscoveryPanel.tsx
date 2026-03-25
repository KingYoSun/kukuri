import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { SettingsEditorField } from './SettingsEditorField';
import { SettingsMetricGrid } from './SettingsMetricGrid';
import { type DiscoveryPanelView } from './types';

type DiscoveryPanelProps = {
  view: DiscoveryPanelView;
  saveDisabled: boolean;
  resetDisabled: boolean;
  onSeedPeersChange: (value: string) => void;
  onSave: () => void;
  onReset: () => void;
};

export function DiscoveryPanel({
  view,
  saveDisabled,
  resetDisabled,
  onSeedPeersChange,
  onSave,
  onReset,
}: DiscoveryPanelProps) {
  return (
    <Card className='space-y-4'>
      <CardHeader>
        <h3>Discovery</h3>
        <small>{view.summaryLabel}</small>
      </CardHeader>

      {view.status === 'loading' ? <Notice>Loading discovery diagnostics…</Notice> : null}
      {view.panelError ? <Notice tone='destructive'>{view.panelError}</Notice> : null}

      <SettingsMetricGrid items={view.metrics} />
      <SettingsDiagnosticList items={view.diagnostics} columns={2} />

      <SettingsEditorField
        label='Seed Peers'
        hint='One endpoint id per line. addr hints stay optional.'
        message={view.seedPeersMessage}
        tone={view.seedPeersMessageTone}
      >
        <Textarea
          aria-label='Seed Peers'
          value={view.seedPeersInput}
          onChange={(event) => onSeedPeersChange(event.target.value)}
          readOnly={view.envLocked}
          className='min-h-[120px] resize-y font-mono text-[0.8rem]'
          placeholder='node_id or node_id@host:port'
        />
      </SettingsEditorField>

      <SettingsActionRow>
        <Button variant='secondary' disabled={saveDisabled} onClick={onSave}>
          Save Seeds
        </Button>
        <Button variant='secondary' disabled={resetDisabled} onClick={onReset}>
          Reset
        </Button>
      </SettingsActionRow>
    </Card>
  );
}
