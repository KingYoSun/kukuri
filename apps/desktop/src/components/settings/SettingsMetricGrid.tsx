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
            item.tone === 'accent' && 'border-[rgba(0,179,164,0.28)] bg-[var(--surface-active)]',
            item.tone === 'warning' && 'border-[rgba(245,157,98,0.28)] bg-[rgba(245,157,98,0.1)]',
            item.tone === 'danger' &&
              'border-[rgba(255,180,138,0.24)] bg-[rgba(255,180,138,0.08)]',
            (!item.tone || item.tone === 'default') &&
              'border-[var(--border-subtle)] bg-white/4'
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
