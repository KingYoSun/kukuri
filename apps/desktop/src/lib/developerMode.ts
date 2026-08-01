export const DEVELOPER_MODE_STORAGE_KEY = 'kukuri.desktop.developer-mode';

export function readDeveloperMode(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  return window.localStorage.getItem(DEVELOPER_MODE_STORAGE_KEY) === 'true';
}

export function writeDeveloperMode(enabled: boolean) {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, enabled ? 'true' : 'false');
}
