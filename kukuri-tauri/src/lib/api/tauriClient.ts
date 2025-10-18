import { invoke } from '@tauri-apps/api/core';

export interface CommandResponse<T> {
  success: boolean;
  data: T | null;
  error?: string | null;
  error_code?: string | null;
}

export class TauriCommandError extends Error {
  code?: string;

  constructor(message: string, code?: string | null) {
    super(message);
    this.name = 'TauriCommandError';
    if (code) {
      this.code = code;
    }
  }
}

export async function invokeCommand<T>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  const response = await invoke<CommandResponse<T>>(command, payload);
  if (!response.success) {
    throw new TauriCommandError(
      response.error ?? `Command ${command} failed`,
      response.error_code,
    );
  }
  return response.data as T;
}

export async function invokeCommandVoid(
  command: string,
  payload?: Record<string, unknown>,
): Promise<void> {
  const response = await invoke<CommandResponse<unknown>>(command, payload);
  if (!response.success) {
    throw new TauriCommandError(
      response.error ?? `Command ${command} failed`,
      response.error_code,
    );
  }
}
