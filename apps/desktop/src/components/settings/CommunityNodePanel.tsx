import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { SettingsEditorField } from './SettingsEditorField';
import { type CommunityNodePanelView } from './types';

type CommunityNodePanelProps = {
  view: CommunityNodePanelView;
  saveDisabled: boolean;
  resetDisabled: boolean;
  clearDisabled: boolean;
  onBaseUrlsChange: (value: string) => void;
  onSaveNodes: () => void;
  onReset: () => void;
  onClearNodes: () => void;
  onAuthenticate: (baseUrl: string) => void;
  onFetchConsents: (baseUrl: string) => void;
  onAcceptConsents: (baseUrl: string) => void;
  onRefresh: (baseUrl: string) => void;
  onClearToken: (baseUrl: string) => void;
};

export function CommunityNodePanel({
  view,
  saveDisabled,
  resetDisabled,
  clearDisabled,
  onBaseUrlsChange,
  onSaveNodes,
  onReset,
  onClearNodes,
  onAuthenticate,
  onFetchConsents,
  onAcceptConsents,
  onRefresh,
  onClearToken,
}: CommunityNodePanelProps) {
  return (
    <Card className='min-w-0 space-y-4'>
      <CardHeader>
        <h3>Community Node</h3>
        <small>{view.summaryLabel}</small>
      </CardHeader>

      {view.status === 'loading' ? <Notice>Loading community node diagnostics…</Notice> : null}
      {view.panelError ? <Notice tone='destructive'>{view.panelError}</Notice> : null}

      <SettingsEditorField
        label='Base URLs'
        hint='One HTTPS base URL per line.'
        message={view.editorMessage}
        tone={view.editorMessageTone}
      >
        <Textarea
          aria-label='Base URLs'
          value={view.baseUrlsInput}
          onChange={(event) => onBaseUrlsChange(event.target.value)}
          className='min-h-[120px] resize-y font-mono text-[0.8rem]'
          placeholder='https://community.example.com'
        />
      </SettingsEditorField>

      <SettingsActionRow>
        <Button variant='secondary' disabled={saveDisabled} onClick={onSaveNodes}>
          Save Nodes
        </Button>
        <Button variant='secondary' disabled={resetDisabled} onClick={onReset}>
          Reset
        </Button>
        <Button variant='secondary' disabled={clearDisabled} onClick={onClearNodes}>
          Clear
        </Button>
      </SettingsActionRow>

      {view.nodes.length === 0 ? <Notice>No community nodes configured.</Notice> : null}

      <div className='min-w-0 space-y-3'>
        {view.nodes.map((node) => (
          <section
            key={node.baseUrl}
            className='min-w-0 rounded-[20px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4 shadow-[0_12px_32px_rgba(2,7,15,0.1)]'
          >
            <div className='flex flex-wrap items-start justify-between gap-3'>
              <div className='min-w-0'>
                <h4 className='break-all text-base font-semibold text-foreground'>{node.baseUrl}</h4>
                <p className='mt-2 text-sm text-[var(--muted-foreground)]'>
                  Auth, consent, and connectivity state for this node.
                </p>
              </div>
            </div>

            <div className='mt-4'>
              <SettingsDiagnosticList items={node.diagnostics} columns={2} />
            </div>

            <div className='mt-4'>
              <SettingsActionRow>
                <Button variant='secondary' onClick={() => onAuthenticate(node.baseUrl)}>
                  Authenticate
                </Button>
                <Button variant='secondary' onClick={() => onFetchConsents(node.baseUrl)}>
                  Consents
                </Button>
                <Button variant='secondary' onClick={() => onAcceptConsents(node.baseUrl)}>
                  Accept
                </Button>
                <Button variant='secondary' onClick={() => onRefresh(node.baseUrl)}>
                  Refresh
                </Button>
                <Button variant='secondary' onClick={() => onClearToken(node.baseUrl)}>
                  Clear Token
                </Button>
              </SettingsActionRow>
            </div>
          </section>
        ))}
      </div>
    </Card>
  );
}
