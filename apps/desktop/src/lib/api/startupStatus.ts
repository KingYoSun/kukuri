import type { DesktopStartupStatus } from './types';

import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

// DesktopApi 外のスタンドアロンコマンド(appConsent.ts と同様に固定スタブを返す)。
export async function getDesktopStartupStatus(): Promise<DesktopStartupStatus> {
  if (isDesktopMockActive()) {
    return { status: 'ready' };
  }
  return invokeDesktop<DesktopStartupStatus>('get_desktop_startup_status');
}
