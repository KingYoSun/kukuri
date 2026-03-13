import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '../../lib/utils';

const noticeVariants = cva('notice', {
  variants: {
    tone: {
      default: '',
      success: 'notice-success',
      error: 'notice-error'
    }
  },
  defaultVariants: {
    tone: 'default'
  }
});

type NoticeProps = React.HTMLAttributes<HTMLDivElement> & VariantProps<typeof noticeVariants>;

const Notice = React.forwardRef<HTMLDivElement, NoticeProps>(({ className, tone, ...props }, ref) => (
  <div ref={ref} className={cn(noticeVariants({ tone }), className)} {...props} />
));
Notice.displayName = 'Notice';

export { Notice };
