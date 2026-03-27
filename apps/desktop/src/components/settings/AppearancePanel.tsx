import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';

import { Card, CardHeader } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';

import { type AppearancePanelView } from './types';

type AppearancePanelProps = {
  view: AppearancePanelView;
  onThemeChange: (theme: AppearancePanelView['selectedTheme']) => void;
  onLocaleChange: (locale: AppearancePanelView['selectedLocale']) => void;
};

export function AppearancePanel({
  view,
  onThemeChange,
  onLocaleChange,
}: AppearancePanelProps) {
  const { t } = useTranslation(['common', 'settings']);

  return (
    <Card className='space-y-4'>
      <CardHeader>
        <h3>{t('settings:appearance.title')}</h3>
        <small>
          {view.selectedTheme === 'dark'
            ? t('settings:appearance.darkSelected')
            : t('settings:appearance.lightSelected')}
        </small>
      </CardHeader>

      <Notice>{t('settings:appearance.themeHint')}</Notice>

      <div
        role='radiogroup'
        aria-label={t('settings:appearance.themeLabel')}
        className='gap-3'
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(14rem, 1fr))',
        }}
      >
        {view.options.map((option) => {
          const selected = option.value === view.selectedTheme;

          return (
            <button
              key={option.value}
              type='button'
              role='radio'
              aria-label={option.label}
              aria-checked={selected}
              className={cn(
                'min-w-0 rounded-[20px] border p-4 text-left transition-colors',
                selected
                  ? 'border-[var(--border-accent)] bg-[var(--surface-active)]'
                  : 'border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] hover:bg-[var(--surface-button-ghost-hover)]'
              )}
              onClick={() => onThemeChange(option.value)}
            >
              <span className='block text-base font-semibold text-foreground'>{option.label}</span>
              <span className='mt-2 block text-sm leading-6 text-[var(--muted-foreground)]'>
                {option.description}
              </span>
              <span
                className={cn(
                  'mt-4 inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-semibold uppercase tracking-[0.08em]',
                  selected
                    ? 'border-[var(--border-accent)] bg-[var(--surface-panel-soft)] text-[var(--accent-foreground)]'
                    : 'border-[var(--border-subtle)] bg-[var(--surface-badge-neutral)] text-[var(--muted-foreground)]'
                )}
              >
                {selected ? t('common:states.active') : t('common:states.available')}
              </span>
            </button>
          );
        })}
      </div>

      <Label>
        <span>{t('settings:appearance.languageLabel')}</span>
        <Select
          aria-label={t('settings:appearance.languageLabel')}
          value={view.selectedLocale}
          onChange={(event) =>
            onLocaleChange(event.target.value as AppearancePanelView['selectedLocale'])
          }
        >
          {view.localeOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </Select>
      </Label>

      <Notice>{t('settings:appearance.languageHint')}</Notice>
    </Card>
  );
}
