import { useCallback, useEffect, useRef } from 'react';

import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { useAuthStore } from '@/stores/authStore';
import { mapApiMessageToModel, useDirectMessageStore } from '@/stores/directMessageStore';

const SYNC_INTERVAL_MS = 30_000;
const MIN_SYNC_INTERVAL_MS = 5_000;

export function useDirectMessageBootstrap() {
  const currentUser = useAuthStore((state) => state.currentUser);
  const isDmSurfaceOpen = useDirectMessageStore(
    (state) => state.isInboxOpen || state.isDialogOpen,
  );
  const hasFetchedRef = useRef(false);
  const lastNpubRef = useRef<string | null>(null);
  const syncLockRef = useRef(false);
  const lastSyncedAtRef = useRef(0);

  const syncConversations = useCallback(
    async ({
      reason,
      force = false,
    }: {
      reason: 'initial' | 'interval' | 'visibility' | 'interaction';
      force?: boolean;
    }) => {
      const currentNpub = currentUser?.npub ?? null;
      if (!currentNpub) {
        return;
      }
      if (syncLockRef.current) {
        return;
      }
      const now = Date.now();
      if (!force && now - lastSyncedAtRef.current < MIN_SYNC_INTERVAL_MS) {
        return;
      }

      syncLockRef.current = true;
      try {
        const response = await TauriApi.listDirectMessageConversations({});
        if (lastNpubRef.current !== currentNpub) {
          return;
        }
        const summaries = response.items.map((item) => ({
          conversationNpub: item.conversationNpub,
          unreadCount: item.unreadCount,
          lastMessage: item.lastMessage ? mapApiMessageToModel(item.lastMessage) : undefined,
        }));

        useDirectMessageStore.getState().hydrateConversations(summaries);
        lastSyncedAtRef.current = Date.now();
        hasFetchedRef.current = true;
      } catch (error) {
        errorHandler.log('DirectMessageBootstrap.fetch_failed', error, {
          context: 'useDirectMessageBootstrap.syncConversations',
          metadata: { reason },
        });
      } finally {
        syncLockRef.current = false;
      }
    },
    [currentUser?.npub],
  );

  useEffect(() => {
    const currentNpub = currentUser?.npub ?? null;
    if (lastNpubRef.current !== currentNpub) {
      lastNpubRef.current = currentNpub;
      hasFetchedRef.current = false;
      lastSyncedAtRef.current = 0;
      syncLockRef.current = false;
    }

    if (!currentNpub) {
      useDirectMessageStore.getState().reset();
      return;
    }

    if (!hasFetchedRef.current) {
      void syncConversations({ reason: 'initial', force: true });
    }
  }, [currentUser?.npub, syncConversations]);

  useEffect(() => {
    if (!currentUser?.npub) {
      return;
    }
    if (typeof window === 'undefined' || typeof document === 'undefined') {
      return;
    }
    const intervalId = window.setInterval(() => {
      void syncConversations({ reason: 'interval' });
    }, SYNC_INTERVAL_MS);

    const handleVisibility = () => {
      if (document.visibilityState === 'visible') {
        void syncConversations({ reason: 'visibility', force: true });
      }
    };

    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [currentUser?.npub, syncConversations]);

  useEffect(() => {
    if (isDmSurfaceOpen) {
      void syncConversations({ reason: 'interaction', force: true });
    }
  }, [isDmSurfaceOpen, syncConversations]);
}
