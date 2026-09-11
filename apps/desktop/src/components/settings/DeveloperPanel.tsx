import { ExternalLink } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type { SettingsSection } from '@/components/shell/types';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { useExternalLinkOpener } from '@/lib/useExternalLinkOpener';
import { SettingsActionRow } from './SettingsActionRow';

// 'release' は開発者モードON時だけ表示される診断レポート(コピー／書き出し)の置き場所。
type DiagnosticSection = Extract<
  SettingsSection,
  'connectivity' | 'discovery' | 'community-node' | 'release'
>;

export const TROUBLESHOOTING_RUNBOOK_URL =
  'https://github.com/KingYoSun/kukuri/blob/main/docs/runbooks/mvp-troubleshooting.md';

type DeveloperPanelProps = {
  developerModeEnabled: boolean;
  onDeveloperModeChange: (enabled: boolean) => void;
  onOpenDiagnostics: (section: DiagnosticSection) => void;
};

export function DeveloperPanel({
  developerModeEnabled,
  onDeveloperModeChange,
  onOpenDiagnostics,
}: DeveloperPanelProps) {
  const { t } = useTranslation(['common', 'settings']);
  const externalLink = useExternalLinkOpener();

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
          {/* #962: 専用ログビューアは無い。所在と取得方法を明示し、存在しない画面を探させない。 */}
          <section className='min-w-0 space-y-2' aria-labelledby='developer-logs-heading'>
            <h4 id='developer-logs-heading' className='text-base font-semibold text-foreground'>
              {t('settings:developer.logs.title')}
            </h4>
            <p className='text-sm text-muted-foreground'>{t('settings:developer.logs.description')}</p>
            <p className='break-words font-mono text-xs text-[var(--muted-foreground-soft)]'>
              {t('settings:developer.logs.capture')}
            </p>
            <p className='text-sm text-muted-foreground'>{t('settings:developer.logs.windows')}</p>
            {externalLink.pending ? <Notice role='status'>{t('common:externalLink.opening')}</Notice> : null}
            {externalLink.failed ? (
              <Notice tone='destructive' role='alert'>{t('common:externalLink.failed')}</Notice>
            ) : null}
            <SettingsActionRow>
              <Button variant='secondary' asChild>
                <a
                  href={TROUBLESHOOTING_RUNBOOK_URL}
                  target='_blank'
                  rel='noreferrer'
                  {...externalLink.linkProps}
                >
                  {t('settings:developer.logs.runbook')}
                  <ExternalLink className='size-4' aria-hidden='true' />
                </a>
              </Button>
            </SettingsActionRow>
          </section>
        </div>
      ) : null}
    </Card>
  );
}
