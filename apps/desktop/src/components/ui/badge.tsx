import * as React from 'react';

import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

const badgeVariants = cva(
  'inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-semibold tracking-[0.08em] uppercase',
  {
    variants: {
      tone: {
        neutral:
          'border-[var(--border-subtle)] bg-[var(--surface-badge-neutral)] text-[var(--muted-foreground)]',
        accent:
          'border-[var(--border-accent)] bg-[var(--surface-accent-soft)] text-[var(--accent-foreground)]',
        warning:
          'border-[var(--border-warning)] bg-[var(--surface-warning-soft)] text-[var(--foreground)]',
        destructive:
          'border-[var(--border-destructive)] bg-[var(--surface-destructive-soft)] text-[var(--destructive)]',
      },
    },
    defaultVariants: {
      tone: 'neutral',
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({ className, tone, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ tone }), className)} {...props} />;
}
