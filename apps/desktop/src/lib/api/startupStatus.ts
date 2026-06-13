import type { DesktopStartupStatus } from './types';

import { invokeDesktop } from './invoke/desktop';

export async function getDesktopStartupStatus(): Promise<DesktopStartupStatus> {
  if (window.__KUKURI_DESKTOP__) {
    return { status: 'ready' };
  }
  return invokeDesktop<DesktopStartupStatus>('get_desktop_startup_status');
}
