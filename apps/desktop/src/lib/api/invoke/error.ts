export const BACKEND_UNAVAILABLE_MESSAGE = 'Desktop backend is not attached.';

export function normalizeInvokeError(error: unknown): Error {
  const normalized =
    error instanceof Error
      ? error
      : typeof error === 'string'
        ? new Error(error)
        : typeof error === 'object' &&
            error !== null &&
            'message' in error &&
            typeof error.message === 'string'
          ? new Error(error.message)
          : new Error(BACKEND_UNAVAILABLE_MESSAGE);
  const message = normalized.message.toLowerCase();
  if (
    message.includes('__tauri') ||
    message.includes('__tauri_ipc__') ||
    (message.includes('ipc') && message.includes('not available')) ||
    (message.includes('invoke') && message.includes('undefined'))
  ) {
    return new Error(BACKEND_UNAVAILABLE_MESSAGE);
  }
  return normalized;
}
