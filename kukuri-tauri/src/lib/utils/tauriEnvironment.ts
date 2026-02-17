export const isTauriRuntime = (): boolean => {
  if (typeof window === 'undefined') {
    return false;
  }

  const candidate = window as Window & {
    __TAURI_INTERNALS__?: { transformCallback?: unknown };
    __TAURI__?: unknown;
    __TAURI_IPC__?: unknown;
  };

  return Boolean(
    candidate.__TAURI_INTERNALS__?.transformCallback ||
    candidate.__TAURI__ ||
    candidate.__TAURI_IPC__,
  );
};
