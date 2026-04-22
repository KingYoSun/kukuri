import { invoke } from '@tauri-apps/api/core';

import { normalizeInvokeError } from './error';

export async function invokeDesktop<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeInvokeError(error);
  }
}
