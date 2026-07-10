import { useEffect, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { CommunityNodeNodeStatus, RuntimeEvent, SyncStatus } from '@/lib/api';
import { isTauriRuntime } from '@/lib/releaseReadiness';

export function useRuntimeEventBridge(
  onNotificationStatusChanged: () => void,
  onSyncStatusChanged: (
    syncStatus: SyncStatus | null,
    communityNodeStatuses: CommunityNodeNodeStatus[] | null
  ) => void
): void {
  const notificationCallbackRef = useRef(onNotificationStatusChanged);
  const syncStatusCallbackRef = useRef(onSyncStatusChanged);

  useEffect(() => {
    notificationCallbackRef.current = onNotificationStatusChanged;
  }, [onNotificationStatusChanged]);

  useEffect(() => {
    syncStatusCallbackRef.current = onSyncStatusChanged;
  }, [onSyncStatusChanged]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    void (async () => {
      const dispose = await listen<RuntimeEvent>(
        'kukuri://runtime-event',
        (event) => {
          switch (event.payload?.type) {
            case 'notification_status_changed':
              notificationCallbackRef.current();
              break;
            case 'sync_status_changed':
              syncStatusCallbackRef.current(
                event.payload.sync_status ?? null,
                event.payload.community_node_statuses ?? null
              );
              break;
          }
        }
      );
      if (cancelled) {
        dispose();
        return;
      }
      unlisten = dispose;
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
}
