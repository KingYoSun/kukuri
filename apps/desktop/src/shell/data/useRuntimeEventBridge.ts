import { useEffect, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import { isTauriRuntime } from '@/lib/releaseReadiness';

type RuntimeEventPayload = {
  type: string;
};

export function useRuntimeEventBridge(
  onNotificationStatusChanged: () => void
): void {
  const callbackRef = useRef(onNotificationStatusChanged);

  useEffect(() => {
    callbackRef.current = onNotificationStatusChanged;
  }, [onNotificationStatusChanged]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    void (async () => {
      const dispose = await listen<RuntimeEventPayload>(
        'kukuri://runtime-event',
        (event) => {
          switch (event.payload?.type) {
            case 'notification_status_changed':
              callbackRef.current();
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
