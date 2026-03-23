import * as React from 'react';

import { cn } from '@/lib/utils';

type FieldProps = React.ComponentProps<'label'> & {
  label: string;
  hint?: string;
  message?: string;
  tone?: 'default' | 'danger';
};

export function Field({
  label,
  hint,
  message,
  tone = 'default',
  className,
  children,
  ...props
}: FieldProps) {
  return (
    <label className={cn('field flex flex-col gap-2', className)} {...props}>
      <span>{label}</span>
      {children}
      {hint ? <small className='text-[0.78rem] text-[var(--muted-foreground-soft)]'>{hint}</small> : null}
      {message ? (
        <small
          className={cn(
            'text-[0.78rem]',
            tone === 'danger' ? 'text-[var(--destructive)]' : 'text-[var(--muted-foreground)]'
          )}
        >
          {message}
        </small>
      ) : null}
    </label>
  );
}
