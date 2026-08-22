import { Pin, PinOff } from 'lucide-react';
import type { ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import type { ColumnSpan } from '@/shell/slices/workspace';

type ColumnSurfaceProps = {
  active: boolean;
  children: ReactNode;
  columnId: string;
  footer?: ReactNode;
  onPinnedChange?: (pinned: boolean) => void;
  pinned: boolean;
  position: number;
  scopeLabel: string;
  span: ColumnSpan;
  title: string;
  total: number;
};

export function ColumnSurface({
  active,
  children,
  columnId,
  footer,
  onPinnedChange,
  pinned,
  position,
  scopeLabel,
  span,
  title,
  total,
}: ColumnSurfaceProps) {
  const stateLabel = pinned ? 'Pinned' : 'Temporary';
  const accessibleLabel = `${title} Column, Column ${position} of ${total}, ${span} span, ${
    active ? 'Active' : 'Inactive'
  }, ${stateLabel}`;

  return (
    <section
      className='shell-column-surface'
      data-active={active || undefined}
      data-column-id={columnId}
      data-pinned={pinned || undefined}
      data-span={span}
      data-transient={!pinned || undefined}
      aria-current={active ? 'true' : undefined}
      aria-label={accessibleLabel}
      aria-roledescription='Column'
      tabIndex={-1}
    >
      <header className='shell-column-header'>
        <div className='shell-column-heading'>
          <div className='shell-column-title-row'>
            <h2>{title}</h2>
            {active ? <span className='shell-column-state-label'>Active</span> : null}
            <span className='shell-column-state-label'>{stateLabel}</span>
          </div>
          <p>{scopeLabel}</p>
        </div>
        {onPinnedChange ? (
          <Button
            variant='ghost'
            size='icon'
            type='button'
            aria-label={pinned ? `Unpin ${title}` : `Pin ${title}`}
            aria-pressed={pinned}
            onClick={() => onPinnedChange(!pinned)}
          >
            {pinned ? (
              <PinOff className='size-4' aria-hidden='true' />
            ) : (
              <Pin className='size-4' aria-hidden='true' />
            )}
          </Button>
        ) : null}
      </header>
      <div className='shell-column-body'>{children}</div>
      {footer ? <footer className='shell-column-footer'>{footer}</footer> : null}
    </section>
  );
}
