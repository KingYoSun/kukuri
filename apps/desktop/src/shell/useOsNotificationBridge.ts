import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

import {
  isTauriRuntime,
  loadOsNotificationSettings,
  OS_NOTIFICATION_SETTINGS_STORAGE_KEY,
} from '@/lib/releaseReadiness';

// OS notification dispatch now lives in the Rust backend (issue #304) so toasts
// fire even while the window is hidden to the tray and regardless of which
// section is open. The frontend's only job here is to mirror the user's
// settings down to the backend dispatcher whenever they change.
export function useOsNotificationBridge(): void {
  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    const syncSettings = () => {
      void invoke('set_os_notification_settings', {
        settings: loadOsNotificationSettings(),
      }).catch(() => {
        // best effort settings sync
      });
    };
    syncSettings();
    window.addEventListener(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, syncSettings);
    return () => {
      window.removeEventListener(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, syncSettings);
    };
  }, []);
}
