import * as React from 'react';

type ShellTopBarProps = {
  activeTopic: string;
};

export function ShellTopBar({ activeTopic }: ShellTopBarProps) {
  return (
    <header className='shell-topbar panel panel-accent' aria-label='Active topic bar'>
      <p className='shell-topbar-topic' title={activeTopic}>
        {activeTopic}
      </p>
    </header>
  );
}
