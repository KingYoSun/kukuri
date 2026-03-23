import * as React from 'react';

import { cn } from '@/lib/utils';

export const Label = React.forwardRef<HTMLLabelElement, React.ComponentProps<'label'>>(
  ({ className, ...props }, ref) => (
    <label ref={ref} className={cn('field flex flex-col gap-2', className)} {...props} />
  )
);

Label.displayName = 'Label';
