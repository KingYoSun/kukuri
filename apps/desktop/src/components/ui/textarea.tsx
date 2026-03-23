import * as React from 'react';

import { cn } from '@/lib/utils';

export const Textarea = React.forwardRef<
  HTMLTextAreaElement,
  React.ComponentProps<'textarea'>
>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={cn(
      'w-full rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-input)] px-4 py-3 text-sm text-foreground placeholder:text-[var(--muted-foreground-soft)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] disabled:cursor-not-allowed disabled:opacity-60',
      className
    )}
    {...props}
  />
));

Textarea.displayName = 'Textarea';
