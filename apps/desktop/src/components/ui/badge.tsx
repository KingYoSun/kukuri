import * as React from 'react';

import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

const badgeVariants = cva(
  'inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-semibold tracking-[0.08em] uppercase',
  {
    variants: {
      tone: {
        neutral:
          'border-[var(--border-subtle)] bg-white/5 text-[var(--muted-foreground)]',
        accent:
          'border-[rgba(0,179,164,0.32)] bg-[rgba(0,179,164,0.14)] text-[var(--accent-foreground)]',
        warning:
          'border-[rgba(245,157,98,0.28)] bg-[rgba(245,157,98,0.16)] text-[var(--foreground)]',
        destructive:
          'border-[rgba(255,180,138,0.24)] bg-[rgba(255,180,138,0.12)] text-[var(--destructive)]',
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
