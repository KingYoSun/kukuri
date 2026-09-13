import * as React from 'react';

import type { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/utils';

export interface ActionRefProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** 実ボタンと同じ lucide icon。icon-only ボタンを指すときに渡す。 */
  icon?: LucideIcon;
  /** 実ボタンと同じ localized な操作名(同じ i18n key から取る)。 */
  label: string;
}

/**
 * 文中で実在する操作ボタンを指す非操作 chip(#994)。
 * 実ボタンの見た目(pill、secondary 面、太字)に合わせ、icon は装飾として隠し、操作名を可視文字で持つ。
 * button role・focus・click を持たないため、案内文の中で「この見た目のボタンを押す」ことだけを伝える。
 */
export function ActionRef({ icon: Icon, label, className, ...props }: ActionRefProps) {
  return (
    <span
      data-testid='action-ref'
      className={cn(
        'inline-flex items-center gap-1 whitespace-nowrap rounded-[var(--radius-pill)] bg-[var(--surface-button-secondary)] px-2 py-0.5 align-baseline text-[0.85em] font-bold leading-tight text-foreground',
        className
      )}
      {...props}
    >
      {Icon ? <Icon className='size-3.5 shrink-0' aria-hidden='true' /> : null}
      <span>{label}</span>
    </span>
  );
}
