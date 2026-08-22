import * as React from 'react';

import { cn } from '@/lib/utils';

type ShellFrameProps = {
  skipTargetId: string;
  navRail?: React.ReactNode;
  workspace: React.ReactNode;
  workspaceLayout?: 'legacy' | 'column';
  detailPaneStack?: React.ReactNode;
  detailPaneCount?: number;
  mobileFooter?: React.ReactNode;
  globalControls?: React.ReactNode;
};

function isMobileViewport() {
  if (typeof window === 'undefined') {
    return false;
  }
  return window.innerWidth <= 759;
}

export function ShellFrame({
  skipTargetId,
  navRail,
  workspace,
  workspaceLayout = 'legacy',
  detailPaneStack,
  detailPaneCount = 0,
  mobileFooter,
  globalControls,
}: ShellFrameProps) {
  const [showMobileFooter, setShowMobileFooter] = React.useState(() => isMobileViewport());
  const layoutDetailPaneCount = Math.max(0, Math.min(detailPaneCount, 2));

  React.useEffect(() => {
    function handleResize() {
      setShowMobileFooter(isMobileViewport());
    }

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
    };
  }, []);

  return (
    <div className='shell-phase1'>
      <a className='shell-skip-link' href={`#${skipTargetId}`}>
        Skip to workspace
      </a>
      <div
        className='shell-layout'
        data-detail-pane-count={layoutDetailPaneCount}
        data-workspace-layout={workspaceLayout}
      >
        {navRail}
        <main
          id={skipTargetId}
          className={cn('shell-main')}
          tabIndex={-1}
          aria-label='Primary workspace'
        >
          <div className='shell-main-lane'>{workspace}</div>
        </main>
        {detailPaneStack}
      </div>
      {mobileFooter && showMobileFooter ? (
        <div className='shell-mobile-footer'>{mobileFooter}</div>
      ) : null}
      {globalControls}
    </div>
  );
}
