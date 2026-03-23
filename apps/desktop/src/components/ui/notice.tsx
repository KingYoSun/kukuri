import * as React from 'react';

import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

const noticeVariants = cva(
  'rounded-[var(--radius-input)] border px-4 py-3 text-sm leading-6 shadow-[0_12px_32px_rgba(2,7,15,0.12)]',
  {
    variants: {
      tone: {
        neutral: 'border-[var(--border-subtle)] bg-white/5 text-foreground',
        accent:
          'border-[rgba(0,179,164,0.24)] bg-[rgba(0,179,164,0.12)] text-[var(--accent-foreground)]',
        warning:
          'border-[rgba(245,157,98,0.24)] bg-[rgba(245,157,98,0.1)] text-foreground',
        destructive:
          'border-[rgba(255,180,138,0.2)] bg-[rgba(255,180,138,0.08)] text-[var(--destructive)]',
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
