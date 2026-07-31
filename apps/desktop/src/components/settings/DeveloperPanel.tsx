import { useTranslation } from 'react-i18next';

import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';

type DeveloperPanelProps = {
  developerModeEnabled: boolean;
  onDeveloperModeChange: (enabled: boolean) => void;
};

export function DeveloperPanel({
  developerModeEnabled,
  onDeveloperModeChange,
}: DeveloperPanelProps) {
  const { t } = useTranslation(['settings']);

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
    </Card>
  );
}
