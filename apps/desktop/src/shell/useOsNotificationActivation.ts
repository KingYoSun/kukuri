import { useCallback, useEffect, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { NotificationView } from '@/lib/api';
import { isTauriRuntime } from '@/lib/releaseReadiness';
import { parseOsNotificationActivationLink } from '@/lib/osNotificationActivationLink';

const CONSUMED_INITIAL_URI_KEY = 'kukuri.os-notification.consumed-initial-uri';

function consumeInitialUri(url: string, initial: boolean): boolean {
  try {
    // getCurrent() retains the last URL across WebView reloads/account switches.
    // Live events always remain actionable, including a repeated click.
    if (initial && sessionStorage.getItem(CONSUMED_INITIAL_URI_KEY) === url) return false;
    sessionStorage.setItem(CONSUMED_INITIAL_URI_KEY, url);
  } catch {
    // Storage failure must not break the active notification click path.
  }
  return true;
}

type ActivationPayload = {
  notification_id: string;
};

/**
 * Resolves native notification events and Windows protocol activations through
 * the current account's notification list and the existing in-app handler.
 *
 * Rust or the single-instance plugin already focuses the window; resolve the
 * id back to a notification and reuse `handleOpenNotification`.
 */
export function useOsNotificationActivation(
  notifications: NotificationView[],
  onActivate: (notification: NotificationView) => void
): void {
  const notificationsRef = useRef<NotificationView[]>(notifications);
  const onActivateRef = useRef(onActivate);
  const pendingIdRef = useRef<string | null>(null);

  const activatePending = useCallback(() => {
    const notification = notificationsRef.current.find(
      (candidate) => candidate.notification_id === pendingIdRef.current
    );
    if (!notification) return;
    // Clear before navigation can trigger another render.
    pendingIdRef.current = null;
    onActivateRef.current(notification);
  }, []);

  useEffect(() => {
    notificationsRef.current = notifications;
    onActivateRef.current = onActivate;
    // Hidden windows pause list refreshes. A native click may arrive before the
    // focus-triggered refresh, so resolve the latest pending click after it.
    activatePending();
  }, [notifications, onActivate, activatePending]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlisten: UnlistenFn | undefined;
    let unlistenUrl: UnlistenFn | undefined;
    let cancelled = false;

    const activate = (notificationId: string | null) => {
      if (cancelled || !notificationId) return;
      pendingIdRef.current = notificationId;
      activatePending();
    };
    const activateUrl = (urls: string[], initial: boolean) => {
      if (cancelled) return;
      // The most recent recognized notification URL wins, just like native clicks.
      for (const url of [...urls].reverse()) {
        const id = parseOsNotificationActivationLink(url);
        if (id !== null) {
          if (consumeInitialUri(url, initial)) activate(id);
          return;
        }
      }
    };

    void (async () => {
      const dispose = await listen<ActivationPayload>('os-notification://activated', (event) => {
        activate(event.payload?.notification_id ?? null);
      });
      if (cancelled) {
        dispose();
        return;
      }
      unlisten = dispose;
    })();

    void import('@tauri-apps/plugin-deep-link').then(async ({ getCurrent, onOpenUrl }) => {
      if (cancelled) return;
      let liveUrlReceived = false;
      const dispose = await onOpenUrl((urls) => {
        liveUrlReceived = true;
        activateUrl(urls, false);
      });
      if (cancelled) {
        dispose();
        return;
      }
      unlistenUrl = dispose;
      // Subscribe before reading the launch URL; a newer live event supersedes it.
      const urls = await getCurrent();
      if (!liveUrlReceived) activateUrl(urls ?? [], true);
    }).catch(() => undefined);

    return () => {
      cancelled = true;
      pendingIdRef.current = null;
      unlisten?.();
      unlistenUrl?.();
    };
  }, [activatePending]);
}
