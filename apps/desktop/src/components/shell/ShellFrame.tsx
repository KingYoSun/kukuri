import * as React from 'react';

import { cn } from '@/lib/utils';

type ShellFrameProps = {
  skipTargetId: string;
  topBar: React.ReactNode;
  navRail: React.ReactNode;
  workspace: React.ReactNode;
  contextPane: React.ReactNode;
};

export function ShellFrame({
  skipTargetId,
  topBar,
  navRail,
  workspace,
  contextPane,
}: ShellFrameProps) {
  return (
    <div className='shell-phase1'>
      <a className='shell-skip-link' href={`#${skipTargetId}`}>
        Skip to workspace
      </a>
      {topBar}
      <div className='shell-layout'>
        {navRail}
        <main
          id={skipTargetId}
          className={cn('shell-main')}
          tabIndex={-1}
          aria-label='Primary workspace'
        >
          {workspace}
        </main>
        {contextPane}
      </div>
    </div>
  );
}
