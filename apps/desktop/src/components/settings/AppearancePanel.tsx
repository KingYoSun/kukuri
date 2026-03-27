import { cn } from '@/lib/utils';

import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';

import { type AppearancePanelView } from './types';

type AppearancePanelProps = {
  view: AppearancePanelView;
  onThemeChange: (theme: AppearancePanelView['selectedTheme']) => void;
};

export function AppearancePanel({ view, onThemeChange }: AppearancePanelProps) {
  return (
    <Card className='space-y-4'>
      <CardHeader>
        <h3>Appearance</h3>
        <small>{view.selectedTheme === 'dark' ? 'dark theme selected' : 'light theme selected'}</small>
      </CardHeader>

      <Notice>Theme changes apply immediately on this device and stay local to this desktop.</Notice>

      <div
        role='radiogroup'
        aria-label='Theme mode'
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
                {selected ? 'Active' : 'Available'}
              </span>
            </button>
          );
        })}
      </div>
    </Card>
  );
}
