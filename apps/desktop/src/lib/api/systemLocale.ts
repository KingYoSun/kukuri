import { isTauri } from '@tauri-apps/api/core';
import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

/** 同意前に許可されたOS言語のlocal read。browser/StorybookではIPCを使わない。 */
export async function getSystemLocales(): Promise<string[]> {
  if (!isTauri() || isDesktopMockActive()) return [];
  return invokeDesktop<string[]>('get_system_locales');
}
