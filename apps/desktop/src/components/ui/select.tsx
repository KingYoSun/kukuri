import * as React from 'react';

import { cn } from '@/lib/utils';

export const Select = React.forwardRef<HTMLSelectElement, React.ComponentProps<'select'>>(
  ({ className, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        'h-11 w-full rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-input)] px-4 py-3 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] disabled:cursor-not-allowed disabled:opacity-60',
        className
      )}
      {...props}
    />
  )
);

Select.displayName = 'Select';
