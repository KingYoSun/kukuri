import * as React from 'react';

import { cn } from '@/lib/utils';

type ShellFrameProps = {
  skipTargetId: string;
  topBar: React.ReactNode;
  navRail: React.ReactNode;
  workspace: React.ReactNode;
  detailPaneStack?: React.ReactNode;
  detailPaneCount?: number;
  mobileFooter?: React.ReactNode;
};

const DETAIL_PANE_COUNT_VAR = '--shell-detail-pane-count' as const;

function isMobileViewport() {
  if (typeof window === 'undefined') {
    return false;
  }
  return window.innerWidth <= 759;
}

export function ShellFrame({
  skipTargetId,
  topBar,
  navRail,
  workspace,
  detailPaneStack,
  detailPaneCount = 0,
  mobileFooter,
}: ShellFrameProps) {
  const [showMobileFooter, setShowMobileFooter] = React.useState(() => isMobileViewport());

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

  const layoutStyle = {
    [DETAIL_PANE_COUNT_VAR]: String(detailPaneCount),
  } as React.CSSProperties;

  return (
    <div className='shell-phase1'>
      <a className='shell-skip-link' href={`#${skipTargetId}`}>
        Skip to workspace
      </a>
      <div className='shell-layout shell-topbar-grid' style={layoutStyle}>
        <div className='shell-topbar-spacer' aria-hidden='true' />
        {topBar}
      </div>
      <div className='shell-layout' style={layoutStyle}>
        {navRail}
        <main
          id={skipTargetId}
          className={cn('shell-main')}
          tabIndex={-1}
          aria-label='Primary workspace'
        >
          {workspace}
        </main>
        {detailPaneStack}
      </div>
      {mobileFooter && showMobileFooter ? (
        <div className='shell-mobile-footer'>{mobileFooter}</div>
      ) : null}
    </div>
  );
}
