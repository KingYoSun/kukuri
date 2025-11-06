import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/authStore';
import {
  useDirectMessageStore,
  type DirectMessageDeliveryStatus,
} from '@/stores/directMessageStore';
import { errorHandler } from '@/lib/errorHandler';

interface DirectMessageEventPayload {
  owner_npub: string;
  conversation_npub: string;
  message: {
    event_id: string | null;
    client_message_id: string | null;
    sender_npub: string;
    recipient_npub: string;
    content: string;
    created_at: number;
    delivered: boolean;
    direction: 'outbound' | 'inbound';
  };
}

export function useDirectMessageEvents() {
  const currentUser = useAuthStore((state) => state.currentUser);

  useEffect(() => {
    if (!currentUser) {
      return;
    }

    if (typeof window === 'undefined') {
      return;
    }

    const tauriWindow = window as Window & {
      __TAURI_INTERNALS__?: { transformCallback?: unknown };
      __TAURI__?: unknown;
      __TAURI_IPC__?: unknown;
    };

    if (
      !tauriWindow.__TAURI_INTERNALS__?.transformCallback &&
      !tauriWindow.__TAURI__ &&
      !tauriWindow.__TAURI_IPC__
    ) {
      return;
    }

    const unlistenPromise = listen<DirectMessageEventPayload>(
      'direct-message:received',
      (event) => {
        try {
          const payload = event.payload;
          if (!payload || payload.owner_npub !== currentUser.npub) {
            return;
          }

          const isOwnMessage = payload.message.sender_npub === currentUser.npub;
          const clientMessageId =
            payload.message.client_message_id ??
            payload.message.event_id ??
            `incoming-${payload.message.created_at}`;

          const status: DirectMessageDeliveryStatus = isOwnMessage
            ? payload.message.delivered
              ? 'sent'
              : 'pending'
            : 'sent';

          useDirectMessageStore.getState().receiveIncomingMessage(
            payload.conversation_npub,
            {
              eventId: payload.message.event_id,
              clientMessageId,
              senderNpub: payload.message.sender_npub,
              recipientNpub: payload.message.recipient_npub,
              content: payload.message.content,
              createdAt: payload.message.created_at,
              status,
            },
            { incrementUnread: !isOwnMessage },
          );
        } catch (error) {
          errorHandler.log('DirectMessageEvents.receive_failed', error, {
            context: 'useDirectMessageEvents',
          });
        }
      },
    );

    return () => {
      unlistenPromise
        .then((unlisten) => {
          unlisten();
        })
        .catch((error) => {
          errorHandler.log('DirectMessageEvents.unlisten_failed', error, {
            context: 'useDirectMessageEvents.cleanup',
          });
        });
    };
  }, [currentUser]);
}
