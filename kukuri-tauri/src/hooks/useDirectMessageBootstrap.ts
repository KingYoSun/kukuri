import { useEffect, useRef } from 'react';

import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { useAuthStore } from '@/stores/authStore';
import { mapApiMessageToModel, useDirectMessageStore } from '@/stores/directMessageStore';

export function useDirectMessageBootstrap() {
  const currentUser = useAuthStore((state) => state.currentUser);
  const hasFetchedRef = useRef(false);
  const lastNpubRef = useRef<string | null>(null);

  useEffect(() => {
    const currentNpub = currentUser?.npub ?? null;
    if (lastNpubRef.current !== currentNpub) {
      hasFetchedRef.current = false;
      lastNpubRef.current = currentNpub;
    }

    if (!currentNpub) {
      useDirectMessageStore.getState().reset();
      hasFetchedRef.current = false;
      return;
    }

    if (hasFetchedRef.current) {
      return;
    }

    let mounted = true;

    (async () => {
      try {
        const response = await TauriApi.listDirectMessageConversations({});
        if (!mounted) {
          return;
        }

        const summaries = response.items.map((item) => ({
          conversationNpub: item.conversationNpub,
          unreadCount: item.unreadCount,
          lastMessage: item.lastMessage ? mapApiMessageToModel(item.lastMessage) : undefined,
        }));

        useDirectMessageStore.getState().hydrateConversations(summaries);
        hasFetchedRef.current = true;
      } catch (error) {
        errorHandler.log('DirectMessageBootstrap.fetch_failed', error, {
          context: 'useDirectMessageBootstrap',
        });
      }
    })();

    return () => {
      mounted = false;
    };
  }, [currentUser?.npub]);
}
