import { cn } from '@/lib/utils';

import { type SettingsMetricView } from './types';

type SettingsMetricGridProps = {
  items: SettingsMetricView[];
};

export function SettingsMetricGrid({ items }: SettingsMetricGridProps) {
  return (
    <dl
      className='gap-3'
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(9rem, 1fr))',
      }}
    >
      {items.map((item) => (
        <div
          key={item.label}
          className={cn(
            'min-w-0 rounded-[18px] border px-4 py-3 shadow-[0_12px_32px_rgba(2,7,15,0.12)]',
            item.tone === 'accent' && 'border-[var(--border-accent)] bg-[var(--surface-active)]',
            item.tone === 'warning' &&
              'border-[var(--border-warning)] bg-[var(--surface-warning-soft)]',
            item.tone === 'danger' &&
              'border-[var(--border-destructive)] bg-[var(--surface-destructive-soft)]',
            (!item.tone || item.tone === 'default') &&
              'border-[var(--border-subtle)] bg-[var(--surface-panel-muted)]'
          )}
        >
          <dt className='text-[0.74rem] uppercase tracking-[0.08em] text-[var(--muted-foreground)]'>
            {item.label}
          </dt>
          <dd
            className={cn(
              'mt-2 break-words text-lg font-semibold text-foreground',
              item.tone === 'danger' && 'text-[var(--destructive)]'
            )}
          >
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}
