import * as React from 'react';

import { PanelLeftOpen, Settings2 } from 'lucide-react';

import { Button } from '@/components/ui/button';

type ShellTopBarProps = {
  headline: string;
  activeTopic: string;
  statusBadges: React.ReactNode;
  navOpen: boolean;
  settingsOpen: boolean;
  navControlsId: string;
  settingsControlsId: string;
  navButtonRef?: React.RefObject<HTMLButtonElement | null>;
  settingsButtonRef?: React.RefObject<HTMLButtonElement | null>;
  onToggleNav: () => void;
  onToggleSettings: () => void;
};

export function ShellTopBar({
  headline,
  activeTopic,
  statusBadges,
  navOpen,
  settingsOpen,
  navControlsId,
  settingsControlsId,
  navButtonRef,
  settingsButtonRef,
  onToggleNav,
  onToggleSettings,
}: ShellTopBarProps) {
  return (
    <header className='shell-topbar panel panel-accent'>
      <div className='shell-topbar-copy'>
        <p className='eyebrow'>kukuri rebuild</p>
        <h1 className='shell-topbar-heading'>{headline}</h1>
        <p className='shell-topbar-topic' title={activeTopic}>
          Active topic: {activeTopic}
        </p>
      </div>
      <div className='shell-topbar-meta'>{statusBadges}</div>
      <div className='shell-topbar-actions'>
        <Button
          ref={navButtonRef}
          variant='ghost'
          size='icon'
          type='button'
          aria-label={navOpen ? 'Close navigation' : 'Open navigation'}
          aria-controls={navControlsId}
          aria-expanded={navOpen}
          data-testid='shell-nav-trigger'
          onClick={onToggleNav}
        >
          <PanelLeftOpen className='size-4' aria-hidden='true' />
        </Button>
        <Button
          ref={settingsButtonRef}
          variant='ghost'
          size='icon'
          type='button'
          aria-label={settingsOpen ? 'Close settings and diagnostics' : 'Open settings and diagnostics'}
          aria-controls={settingsControlsId}
          aria-expanded={settingsOpen}
          data-testid='shell-settings-trigger'
          onClick={onToggleSettings}
        >
          <Settings2 className='size-4' aria-hidden='true' />
        </Button>
      </div>
    </header>
  );
}
