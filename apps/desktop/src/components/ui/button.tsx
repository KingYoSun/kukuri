import * as React from 'react';

import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';

import { cn } from '@/lib/utils';

const buttonVariants = cva(
  'button inline-flex items-center justify-center gap-2 whitespace-nowrap transition-colors disabled:pointer-events-none',
  {
    variants: {
      variant: {
        primary: 'shadow-[0_10px_28px_rgba(245,157,98,0.16)]',
        secondary: 'button-secondary',
        ghost:
          'border border-[var(--border-subtle)] bg-transparent text-foreground shadow-none hover:bg-white/5',
      },
      size: {
        default: 'min-h-11 px-4 py-3',
        sm: 'min-h-9 px-3 py-2 text-sm',
        icon: 'size-10 p-0',
      },
    },
    defaultVariants: {
      variant: 'primary',
      size: 'default',
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, type = 'button', ...props }, ref) => {
    const Comp = asChild ? Slot : 'button';

    return (
      <Comp
        ref={ref}
        className={cn(buttonVariants({ variant, size }), className)}
        type={type}
        {...props}
      />
    );
  }
);

Button.displayName = 'Button';
