import type * as React from 'react';

import { cn } from '@/lib/utils';

type SettingsStoryFrameProps = {
  children: React.ReactNode;
  width?: 'wide' | 'narrow';
};

export function SettingsStoryFrame({
  children,
  width = 'wide',
}: SettingsStoryFrameProps) {
  return (
    <div className='min-h-screen bg-[var(--shell-background)] p-6 text-foreground'>
      <div
        data-figma-capture-root
        className={cn(
          'mx-auto isolate overflow-hidden rounded-[32px] bg-[var(--shell-background)]',
          width === 'wide' ? 'w-full max-w-[1120px]' : 'w-full max-w-[480px]'
        )}
      >
        {children}
      </div>
    </div>
  );
}
