import { cn } from '@/lib/utils';

import { type SettingsDiagnosticItemView } from './types';

type SettingsDiagnosticListProps = {
  items: SettingsDiagnosticItemView[];
  columns?: 1 | 2;
};

export function SettingsDiagnosticList({
  items,
  columns = 1,
}: SettingsDiagnosticListProps) {
  return (
    <dl
      className='gap-3'
      style={
        columns === 2
          ? {
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(16rem, 1fr))',
            }
          : { display: 'grid' }
      }
    >
      {items.map((item) => (
        <div
          key={item.label}
          className='min-w-0 rounded-[18px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] px-4 py-3 shadow-[0_12px_32px_rgba(2,7,15,0.1)]'
        >
          <dt className='text-[0.74rem] uppercase tracking-[0.08em] text-[var(--muted-foreground)]'>
            {item.label}
          </dt>
          <dd
            className={cn(
              'mt-2 min-w-0 break-words text-sm leading-6 text-[var(--muted-foreground-soft)]',
              item.monospace && 'break-all font-mono text-[0.8rem]',
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
