import { useTranslation } from 'react-i18next';

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
  const { t } = useTranslation(['common', 'settings']);

  return (
    <Card className='min-w-0 space-y-4'>
      <CardHeader>
        <h3>{t('settings:communityNode.title')}</h3>
        <small>{view.summaryLabel}</small>
      </CardHeader>

      {view.status === 'loading' ? <Notice>{t('settings:communityNode.loading')}</Notice> : null}
      {view.panelError ? <Notice tone='destructive'>{view.panelError}</Notice> : null}

      <SettingsEditorField
        label={t('settings:communityNode.baseUrlsLabel')}
        hint={t('settings:communityNode.baseUrlsHint')}
        message={view.editorMessage}
        tone={view.editorMessageTone}
      >
        <Textarea
          aria-label={t('settings:communityNode.baseUrlsLabel')}
          value={view.baseUrlsInput}
          onChange={(event) => onBaseUrlsChange(event.target.value)}
          className='min-h-[120px] resize-y font-mono text-[0.8rem]'
          placeholder={t('settings:communityNode.baseUrlsPlaceholder')}
        />
      </SettingsEditorField>

      <SettingsActionRow>
        <Button variant='secondary' disabled={saveDisabled} onClick={onSaveNodes}>
          {t('settings:communityNode.actions.saveNodes')}
        </Button>
        <Button variant='secondary' disabled={resetDisabled} onClick={onReset}>
          {t('common:actions.reset')}
        </Button>
        <Button variant='secondary' disabled={clearDisabled} onClick={onClearNodes}>
          {t('settings:communityNode.actions.clearNodes')}
        </Button>
      </SettingsActionRow>

      {view.nodes.length === 0 ? <Notice>{t('settings:communityNode.noNodes')}</Notice> : null}

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
                  {t('settings:communityNode.nodeSummary')}
                </p>
              </div>
            </div>

            <div className='mt-4'>
              <SettingsDiagnosticList items={node.diagnostics} columns={2} />
            </div>

            <div className='mt-4'>
              <SettingsActionRow>
                <Button variant='secondary' onClick={() => onAuthenticate(node.baseUrl)}>
                  {t('common:actions.authenticate')}
                </Button>
                <Button variant='secondary' onClick={() => onFetchConsents(node.baseUrl)}>
                  {t('common:actions.consents')}
                </Button>
                <Button variant='secondary' onClick={() => onAcceptConsents(node.baseUrl)}>
                  {t('common:actions.accept')}
                </Button>
                <Button variant='secondary' onClick={() => onRefresh(node.baseUrl)}>
                  {t('common:actions.refresh')}
                </Button>
                <Button variant='secondary' onClick={() => onClearToken(node.baseUrl)}>
                  {t('settings:communityNode.actions.clearToken')}
                </Button>
              </SettingsActionRow>
            </div>
          </section>
        ))}
      </div>
    </Card>
  );
}
