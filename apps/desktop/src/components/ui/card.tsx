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

type CardOwnProps<T extends React.ElementType> = {
  as?: T;
} & VariantProps<typeof cardVariants>;

type CardProps<T extends React.ElementType> = CardOwnProps<T> &
  Omit<React.ComponentPropsWithoutRef<T>, keyof CardOwnProps<T>>;

export function Card<T extends React.ElementType = 'section'>({
  as,
  tone,
  className,
  ...props
}: CardProps<T>) {
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
