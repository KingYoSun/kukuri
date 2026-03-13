import { useMemo } from 'react';
import { useDirectMessageStore, type DirectMessageModel } from '@/stores/directMessageStore';

interface DirectMessageBadgeData {
  unreadTotal: number;
  latestConversationNpub: string | null;
  latestMessage: DirectMessageModel | null;
}

export function useDirectMessageBadge(): DirectMessageBadgeData {
  const unreadCounts = useDirectMessageStore((state) => state.unreadCounts);
  const conversations = useDirectMessageStore((state) => state.conversations);

  return useMemo(() => {
    const unreadTotal = Object.values(unreadCounts).reduce((sum, value) => sum + value, 0);

    let latestConversationNpub: string | null = null;
    let latestMessage: DirectMessageModel | null = null;

    for (const [npub, messages] of Object.entries(conversations)) {
      if (!messages || messages.length === 0) continue;
      const candidate = messages[messages.length - 1];
      if (!latestMessage || candidate.createdAt > latestMessage.createdAt) {
        latestConversationNpub = npub;
        latestMessage = candidate;
      }
    }

    return {
      unreadTotal,
      latestConversationNpub,
      latestMessage,
    };
  }, [conversations, unreadCounts]);
}
