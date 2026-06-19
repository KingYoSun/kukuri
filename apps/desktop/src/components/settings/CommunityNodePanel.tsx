import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';

import { SettingsActionRow } from './SettingsActionRow';
import { SettingsDiagnosticList } from './SettingsDiagnosticList';
import { SettingsEditorField } from './SettingsEditorField';
import { type CommunityNodePanelView } from './types';

type CommunityNodePanelProps = {
  view: CommunityNodePanelView;
  saveDisabled: boolean;
  resetDisabled: boolean;
  clearDisabled: boolean;
  nodeActionsDisabled?: boolean;
  onAddNode: () => void;
  onNodeBaseUrlChange: (id: string, value: string) => void;
  onNodeAutoApproveChange: (id: string, value: boolean) => void;
  onRemoveNode: (id: string) => void;
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
  nodeActionsDisabled = false,
  onAddNode,
  onNodeBaseUrlChange,
  onNodeAutoApproveChange,
  onRemoveNode,
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
        label={t('settings:communityNode.nodesLabel')}
        hint={t('settings:communityNode.nodesHint')}
        message={view.editorMessage}
        tone={view.editorMessageTone}
      >
        <SettingsActionRow>
          <Button variant='secondary' onClick={onAddNode}>
            {t('settings:communityNode.actions.addNode')}
          </Button>
        </SettingsActionRow>
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
            key={node.id}
            className='min-w-0 rounded-[20px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4 shadow-[var(--shadow-dropdown)]'
          >
            <div className='flex flex-wrap items-start justify-between gap-3'>
              <div className='min-w-0 flex-1 space-y-3'>
                <h4 className='break-all text-base font-semibold text-foreground'>
                  {node.baseUrl.trim() || t('settings:communityNode.baseUrlsPlaceholder')}
                </h4>
                <div className='space-y-2'>
                  <label className='block text-sm font-medium text-foreground'>
                    {t('settings:communityNode.baseUrlLabel')}
                  </label>
                  <Input
                    aria-label={t('settings:communityNode.baseUrlLabel')}
                    value={node.baseUrl}
                    onChange={(event) => onNodeBaseUrlChange(node.id, event.target.value)}
                    placeholder={t('settings:communityNode.baseUrlsPlaceholder')}
                    className='font-mono text-[0.8rem]'
                  />
                </div>
                <label className='flex items-center gap-3 text-sm text-foreground'>
                  <input
                    type='checkbox'
                    checked={node.autoApprove}
                    onChange={(event) =>
                      onNodeAutoApproveChange(node.id, event.currentTarget.checked)
                    }
                  />
                  <span>{t('settings:communityNode.autoApproveLabel')}</span>
                </label>
                <p className='text-sm text-[var(--muted-foreground)]'>
                  {node.saved
                    ? t('settings:communityNode.nodeSummary')
                    : t('settings:communityNode.unsavedNodeSummary')}
                </p>
              </div>
              <Button variant='secondary' onClick={() => onRemoveNode(node.id)}>
                {t('common:actions.remove')}
              </Button>
            </div>

            <div className='mt-4'>
              <SettingsDiagnosticList items={node.diagnostics} columns={2} />
            </div>

            <div className='mt-4 space-y-2'>
              <h5 className='text-sm font-semibold text-foreground'>
                {t('settings:communityNode.dependency.heading')}
              </h5>
              <SettingsDiagnosticList items={node.dependency.diagnostics} columns={2} />
              {node.dependency.manifestError ? (
                <Notice tone='destructive'>{node.dependency.manifestError}</Notice>
              ) : null}
              {node.dependency.boundaryNotes.map((note) => (
                <p key={note} className='text-sm text-[var(--muted-foreground)]'>
                  {note}
                </p>
              ))}
            </div>

            <div className='mt-4'>
              <SettingsActionRow>
                <Button
                  variant='secondary'
                  disabled={nodeActionsDisabled || !node.saved || !node.baseUrl.trim()}
                  onClick={() => onAuthenticate(node.baseUrl)}
                >
                  {t('common:actions.authenticate')}
                </Button>
                <Button
                  variant='secondary'
                  disabled={nodeActionsDisabled || !node.saved || !node.baseUrl.trim()}
                  onClick={() => onFetchConsents(node.baseUrl)}
                >
                  {t('common:actions.consents')}
                </Button>
                <Button
                  variant='secondary'
                  disabled={nodeActionsDisabled || !node.saved || !node.baseUrl.trim()}
                  onClick={() => onAcceptConsents(node.baseUrl)}
                >
                  {t('common:actions.accept')}
                </Button>
                <Button
                  variant='secondary'
                  disabled={nodeActionsDisabled || !node.saved || !node.baseUrl.trim()}
                  onClick={() => onRefresh(node.baseUrl)}
                >
                  {t('common:actions.refresh')}
                </Button>
                <Button
                  variant='secondary'
                  disabled={nodeActionsDisabled || !node.saved || !node.baseUrl.trim()}
                  onClick={() => onClearToken(node.baseUrl)}
                >
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
