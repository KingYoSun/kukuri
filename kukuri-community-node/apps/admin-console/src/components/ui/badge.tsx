import * as React from 'react';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '../../lib/utils';

const badgeVariants = cva('badge', {
  variants: {
    tone: {
      default: '',
      good: 'good',
      warn: 'warn',
      bad: 'bad'
    }
  },
  defaultVariants: {
    tone: 'default'
  }
});

type BadgeProps = React.HTMLAttributes<HTMLSpanElement> & VariantProps<typeof badgeVariants>;

const Badge = React.forwardRef<HTMLSpanElement, BadgeProps>(({ className, tone, ...props }, ref) => (
  <span ref={ref} className={cn(badgeVariants({ tone }), className)} {...props} />
));
Badge.displayName = 'Badge';

export { Badge, badgeVariants };
