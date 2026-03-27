import * as React from 'react';

import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

const noticeVariants = cva(
  'rounded-[var(--radius-input)] border px-4 py-3 text-sm leading-6 shadow-[0_12px_32px_rgba(2,7,15,0.12)]',
  {
    variants: {
      tone: {
        neutral: 'border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] text-foreground',
        accent:
          'border-[var(--border-accent)] bg-[var(--surface-accent-soft)] text-[var(--accent-foreground)]',
        warning:
          'border-[var(--border-warning)] bg-[var(--surface-warning-soft)] text-foreground',
        destructive:
          'border-[var(--border-destructive)] bg-[var(--surface-destructive-soft)] text-[var(--destructive)]',
      },
    },
    defaultVariants: {
      tone: 'neutral',
    },
  }
);

export interface NoticeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof noticeVariants> {}

export function Notice({ className, tone, ...props }: NoticeProps) {
  return <div className={cn(noticeVariants({ tone }), className)} {...props} />;
}
