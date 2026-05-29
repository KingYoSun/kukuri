import * as React from 'react';

import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

const cardVariants = cva('panel', {
  variants: {
    tone: {
      default: '',
      accent: 'panel-accent',
    },
  },
  defaultVariants: {
    tone: 'default',
  },
});

type CardElement = 'article' | 'aside' | 'div' | 'section';

type CardProps = {
  as?: CardElement;
} & VariantProps<typeof cardVariants> &
  Omit<React.ComponentPropsWithoutRef<CardElement>, 'as'>;

export function Card({
  as,
  tone,
  className,
  ...props
}: CardProps) {
  const Component = as ?? 'section';

  return <Component className={cn(cardVariants({ tone }), className)} {...props} />;
}

export function CardHeader({ className, ...props }: React.ComponentProps<'div'>) {
  return <div className={cn('panel-header', className)} {...props} />;
}

export function CardContent({ className, ...props }: React.ComponentProps<'div'>) {
  return <div className={cn(className)} {...props} />;
}

export function CardTitle({ className, ...props }: React.ComponentProps<'h3'>) {
  return <h3 className={cn(className)} {...props} />;
}
