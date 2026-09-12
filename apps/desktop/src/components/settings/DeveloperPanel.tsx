import type * as React from 'react';

import { useTranslation } from 'react-i18next';

import type { SettingsSection } from '@/components/shell/types';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { SettingsActionRow } from './SettingsActionRow';

// 'release' は開発者モードON時だけ表示される診断レポート(コピー／書き出し)の置き場所。
type DiagnosticSection = Extract<
  SettingsSection,
  'connectivity' | 'discovery' | 'community-node' | 'release'
>;

type DeveloperPanelProps = {
  developerModeEnabled: boolean;
  onDeveloperModeChange: (enabled: boolean) => void;
  onOpenDiagnostics: (section: DiagnosticSection) => void;
  /// #978: 開発者モードON時だけ描画するログビューア(`DeveloperLogViewer`)。取得は呼出元が担う。
  logs?: React.ReactNode;
};

export function DeveloperPanel({
  developerModeEnabled,
  onDeveloperModeChange,
  onOpenDiagnostics,
  logs,
}: DeveloperPanelProps) {
  const { t } = useTranslation(['common', 'settings']);

  return (
    <Card className='space-y-4'>
      <CardHeader>
        <h3>{t('settings:developer.title')}</h3>
        <small>{t('settings:developer.summary')}</small>
      </CardHeader>

      <label className='flex min-w-0 items-center gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] px-4 py-3 text-sm text-foreground'>
        <input
          type='checkbox'
          checked={developerModeEnabled}
          onChange={(event) => onDeveloperModeChange(event.currentTarget.checked)}
        />
        <span>{t('settings:developer.mode.label')}</span>
      </label>

      <Notice>{t('settings:developer.mode.description')}</Notice>
      <p role='status' aria-atomic='true' className='text-sm'>
        {t(developerModeEnabled ? 'settings:developer.mode.enabled' : 'settings:developer.mode.disabled')}
      </p>
      {developerModeEnabled ? (
        <div className='space-y-3'>
          <p className='text-sm text-muted-foreground'>{t('settings:developer.diagnostics.description')}</p>
          <SettingsActionRow className='flex-col'>
            {(['connectivity', 'discovery', 'community-node', 'release'] as const).map((section) => (
              <Button
                key={section}
                type='button'
                variant='secondary'
                className='h-auto min-h-11 whitespace-normal text-left'
                onClick={() => onOpenDiagnostics(section)}
              >
                {t(`settings:developer.diagnostics.${section}`)}
              </Button>
            ))}
          </SettingsActionRow>
          {/* #978: OFF 時はこの slot ごと unmount され、ログ取得も起きない。 */}
          {logs}
        </div>
      ) : null}
    </Card>
  );
}
