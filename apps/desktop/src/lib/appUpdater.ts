import { Channel, invoke } from '@tauri-apps/api/core';
import type { DownloadEvent } from '@tauri-apps/plugin-updater';

// No target, URL, key, bytes or installer arguments come from the webview.
export async function check() {
  const update = await invoke<{ id: number; version: string } | null>('check_app_update');
  if (!update) return null;
  return {
    version: update.version,
    download: async (onEvent?: (event: DownloadEvent) => void) => {
      const channel = new Channel<DownloadEvent>();
      channel.onmessage = (event) => onEvent?.(event);
      await invoke('download_app_update', { id: update.id, onEvent: channel });
    },
    install: async () => { await invoke('install_app_update', { id: update.id }); },
  };
}
