import { describe, it, beforeEach, expect } from 'vitest';
import { renderHook } from '@testing-library/react';

import { useDirectMessageBadge } from '@/hooks/useDirectMessageBadge';
import { useDirectMessageStore, getDirectMessageInitialState } from '@/stores/directMessageStore';

describe('useDirectMessageBadge', () => {
  beforeEach(() => {
    useDirectMessageStore.setState(getDirectMessageInitialState());
  });

  it('未読件数と最新メッセージを返す', () => {
    const now = Date.now();
    useDirectMessageStore.setState((state) => ({
      ...state,
      conversations: {
        npubOld: [
          {
            eventId: 'evt-1',
            clientMessageId: 'client-1',
            senderNpub: 'npubOld',
            recipientNpub: 'npubSelf',
            content: '古いメッセージ',
            createdAt: now - 10_000,
            status: 'sent',
          },
        ],
        npubNew: [
          {
            eventId: 'evt-2',
            clientMessageId: 'client-2',
            senderNpub: 'npubNew',
            recipientNpub: 'npubSelf',
            content: '最新メッセージ',
            createdAt: now,
            status: 'sent',
          },
        ],
      },
      unreadCounts: {
        npubOld: 1,
        npubNew: 4,
      },
    }));

    const { result } = renderHook(() => useDirectMessageBadge());

    expect(result.current.unreadTotal).toBe(5);
    expect(result.current.latestConversationNpub).toBe('npubNew');
    expect(result.current.latestMessage?.content).toBe('最新メッセージ');
  });

  it('会話が無い場合はデフォルト値を返す', () => {
    const { result } = renderHook(() => useDirectMessageBadge());

    expect(result.current.unreadTotal).toBe(0);
    expect(result.current.latestConversationNpub).toBeNull();
    expect(result.current.latestMessage).toBeNull();
  });
});
