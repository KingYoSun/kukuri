import { useCallback, useEffect, useState } from 'react';

import {
  getOsNotificationPermission,
  requestOsNotificationPermission,
} from '@/lib/api/osNotificationPermission';
import {
  DEFAULT_OS_NOTIFICATION_SETTINGS,
  isTauriRuntime,
  loadOsNotificationSettings,
  OS_NOTIFICATION_SETTINGS_STORAGE_KEY,
  saveOsNotificationSettings,
  type OsNotificationSettings,
} from '@/lib/releaseReadiness';

// #962: OS 通知設定の読み書きと権限確認を、通知設定パネルと診断レポートで共有する。
// 保存 key / 既定値 / backend への mirror は releaseReadiness と useOsNotificationBridge のまま。
export function useOsNotificationSettings(): [
  OsNotificationSettings,
  (patch: Partial<OsNotificationSettings>) => void,
] {
  const [settings, setSettings] = useState<OsNotificationSettings>(DEFAULT_OS_NOTIFICATION_SETTINGS);

  useEffect(() => {
    const sync = () => setSettings(loadOsNotificationSettings());
    sync();
    window.addEventListener(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, sync);
    return () => window.removeEventListener(OS_NOTIFICATION_SETTINGS_STORAGE_KEY, sync);
  }, []);

  const update = useCallback(
    (patch: Partial<OsNotificationSettings>) => {
      const next = { ...settings, ...patch };
      setSettings(next);
      saveOsNotificationSettings(next);
    },
    [settings]
  );

  return [settings, update];
}

export type OsNotificationPermissionState = {
  permission: string;
  checking: boolean;
  /** 明示的な再確認。結果の正規化済み permission を返し、失敗時は `unavailable` にする。 */
  request: () => Promise<string | null>;
};

export function useOsNotificationPermission(): OsNotificationPermissionState {
  const [permission, setPermission] = useState('unknown');
  const [checking, setChecking] = useState(isTauriRuntime);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    let cancelled = false;
    // Query the Tauri backend directly instead of the WebView Web Notification
    // API, whose permission state is volatile and unreliable on Windows (#313).
    void getOsNotificationPermission()
      .then((value) => {
        if (!cancelled) {
          setPermission(value.toLowerCase());
        }
      })
      .catch(() => {
        if (!cancelled) {
          setPermission('unavailable');
        }
      })
      .finally(() => {
        if (!cancelled) {
          setChecking(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const request = useCallback(async () => {
    if (checking) {
      return null;
    }
    if (!isTauriRuntime()) {
      setPermission('unavailable');
      return null;
    }
    setChecking(true);
    try {
      const normalized = (await requestOsNotificationPermission()).toLowerCase();
      setPermission(normalized);
      return normalized;
    } catch {
      setPermission('unavailable');
      return null;
    } finally {
      setChecking(false);
    }
  }, [checking]);

  return { permission, checking, request };
}
