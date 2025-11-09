import { invoke } from '@tauri-apps/api/core';

export interface CommandResponse<T> {
  success: boolean;
  data: T | null;
  error?: string | null;
  error_code?: string | null;
  error_details?: Record<string, unknown> | null;
}

export class TauriCommandError extends Error {
  code?: string;
  details?: Record<string, unknown> | null;

  constructor(message: string, code?: string | null, details?: Record<string, unknown> | null) {
    super(message);
    this.name = 'TauriCommandError';
    if (code) {
      this.code = code;
    }
    if (details) {
      this.details = details;
    }
  }
}

function isCommandResponse<T>(value: unknown): value is CommandResponse<T> {
  return (
    typeof value === 'object' &&
    value !== null &&
    'success' in value &&
    typeof (value as CommandResponse<unknown>).success === 'boolean'
  );
}

async function tauriInvoke<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  if (payload === undefined) {
    return invoke<T>(command);
  }
  return invoke<T>(command, payload);
}

export async function invokeCommand<T>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  const response = await tauriInvoke<CommandResponse<T>>(command, payload);
  if (isCommandResponse<T>(response)) {
    if (!response.success) {
      throw new TauriCommandError(
        response.error ?? `Command ${command} failed`,
        response.error_code,
        response.error_details ?? null,
      );
    }
    return response.data as T;
  }
  if (response !== undefined) {
    return response as T;
  }
  throw new TauriCommandError(`Command ${command} returned no response`);
}

export async function invokeCommandVoid(
  command: string,
  payload?: Record<string, unknown>,
): Promise<void> {
  const response = await tauriInvoke<CommandResponse<unknown>>(command, payload);
  if (response === undefined || response === null) {
    return;
  }
  if (!isCommandResponse<unknown>(response)) {
    return;
  }
  if (!response.success) {
    throw new TauriCommandError(
      response.error ?? `Command ${command} failed`,
      response.error_code,
      response.error_details ?? null,
    );
  }
}
