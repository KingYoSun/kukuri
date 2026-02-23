import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useRealtimeTimeline } from '@/hooks/useRealtimeTimeline';
import { TIMELINE_REALTIME_DELTA_EVENT } from '@/lib/realtime/timelineRealtimeEvents';
import { useP2PStore } from '@/stores/p2pStore';
import type { TopicTimelineEntry } from '@/hooks/usePosts';
import type { NostrEventPayload } from '@/types/nostr';

const buildQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

const buildWrapper =
  (queryClient: QueryClient) =>
  ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

const baseTimelineEntry: TopicTimelineEntry = {
  threadUuid: 'thread-1',
  parentPost: {
    id: 'parent-1',
    content: 'parent',
    author: {
      id: 'author-1',
      pubkey: 'author-1',
      npub: 'npub-author-1',
      name: 'Author1',
      displayName: 'Author1',
      picture: '',
      about: '',
      nip05: '',
      avatar: null,
      publicProfile: true,
      showOnlineStatus: false,
    },
    topicId: 'topic-1',
    threadNamespace: 'topic-1/threads/thread-1',
    threadUuid: 'thread-1',
    threadRootEventId: 'parent-1',
    threadParentEventId: null,
    created_at: 1_700_000_000,
    tags: [],
    likes: 0,
    boosts: 0,
    replies: [],
    replyCount: 0,
    isSynced: true,
  },
  firstReply: null,
  replyCount: 0,
  lastActivityAt: 1_700_000_000,
};

const dispatchRealtimeDelta = (detail: unknown) => {
  window.dispatchEvent(
    new CustomEvent(TIMELINE_REALTIME_DELTA_EVENT, {
      detail,
    }),
  );
};

describe('useRealtimeTimeline', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useP2PStore.setState({ connectionStatus: 'connected' });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('nostr realtime delta をバッチ適用して timeline cache を更新する', () => {
    const queryClient = buildQueryClient();
    queryClient.setQueryData<TopicTimelineEntry[]>(
      ['topicTimeline', 'topic-1'],
      [baseTimelineEntry],
    );
    const onFallbackToStandard = vi.fn();

    renderHook(
      () =>
        useRealtimeTimeline({
          topicId: 'topic-1',
          mode: 'realtime',
          onFallbackToStandard,
        }),
      { wrapper: buildWrapper(queryClient) },
    );

    const newRootPayload: NostrEventPayload = {
      id: 'parent-2',
      author: 'author-2',
      content: 'new thread root',
      created_at: 1_700_000_050,
      kind: 30078,
      tags: [
        ['t', 'topic-1'],
        ['thread', 'topic-1/threads/thread-2'],
        ['thread_uuid', 'thread-2'],
      ],
    };

    const replyPayload: NostrEventPayload = {
      id: 'reply-1',
      author: 'author-3',
      content: 'reply',
      created_at: 1_700_000_100,
      kind: 30078,
      tags: [
        ['t', 'topic-1'],
        ['thread', 'topic-1/threads/thread-1'],
        ['thread_uuid', 'thread-1'],
        ['e', 'parent-1', '', 'root'],
        ['e', 'parent-1', '', 'reply'],
      ],
    };

    act(() => {
      dispatchRealtimeDelta({
        source: 'nostr',
        payload: newRootPayload,
        receivedAt: Date.now(),
      });
      dispatchRealtimeDelta({
        source: 'nostr',
        payload: replyPayload,
        receivedAt: Date.now(),
      });
      vi.advanceTimersByTime(750);
    });

    const updated = queryClient.getQueryData<TopicTimelineEntry[]>(['topicTimeline', 'topic-1']);
    expect(updated).toHaveLength(2);
    expect(updated?.[0].threadUuid).toBe('thread-1');
    expect(updated?.[0].replyCount).toBe(1);
    expect(updated?.[0].firstReply?.id).toBe('reply-1');
    expect(updated?.[1].threadUuid).toBe('thread-2');
    expect(onFallbackToStandard).not.toHaveBeenCalled();
  });

  it('適用不能な差分はバッチ後に1回だけ refetch を要求する', () => {
    const queryClient = buildQueryClient();
    queryClient.setQueryData<TopicTimelineEntry[]>(
      ['topicTimeline', 'topic-1'],
      [baseTimelineEntry],
    );
    const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries');

    renderHook(
      () =>
        useRealtimeTimeline({
          topicId: 'topic-1',
          mode: 'realtime',
          onFallbackToStandard: vi.fn(),
        }),
      { wrapper: buildWrapper(queryClient) },
    );

    const unresolvedReplyPayload: NostrEventPayload = {
      id: 'reply-unknown',
      author: 'author-4',
      content: 'unknown reply',
      created_at: 1_700_000_200,
      kind: 30078,
      tags: [
        ['t', 'topic-1'],
        ['thread', 'topic-1/threads/thread-unknown'],
        ['thread_uuid', 'thread-unknown'],
        ['e', 'missing-root', '', 'root'],
        ['e', 'missing-parent', '', 'reply'],
      ],
    };

    act(() => {
      dispatchRealtimeDelta({
        source: 'nostr',
        payload: unresolvedReplyPayload,
        receivedAt: Date.now(),
      });
      dispatchRealtimeDelta({
        source: 'p2p',
        topicId: 'topic-1',
        message: {
          id: 'p2p-message-1',
          topic_id: 'topic-1',
          author: 'author-p2p',
          content: 'p2p payload',
          timestamp: Date.now(),
          signature: 'sig',
        },
        receivedAt: Date.now(),
      });
      vi.advanceTimersByTime(750);
    });

    expect(invalidateQueriesSpy).toHaveBeenCalledTimes(1);
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({
      queryKey: ['topicTimeline', 'topic-1'],
    });
  });

  it('トピック切り替え時に pending flush timer を破棄して新トピック差分を適用する', () => {
    const queryClient = buildQueryClient();
    queryClient.setQueryData<TopicTimelineEntry[]>(
      ['topicTimeline', 'topic-1'],
      [baseTimelineEntry],
    );

    const topic2BaseEntry: TopicTimelineEntry = {
      ...baseTimelineEntry,
      threadUuid: 'topic2-thread-1',
      parentPost: {
        ...baseTimelineEntry.parentPost,
        id: 'topic2-parent-1',
        topicId: 'topic-2',
        threadNamespace: 'topic-2/threads/topic2-thread-1',
        threadUuid: 'topic2-thread-1',
        threadRootEventId: 'topic2-parent-1',
      },
      lastActivityAt: 1_700_000_010,
    };
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicTimeline', 'topic-2'], [topic2BaseEntry]);

    const { rerender } = renderHook(
      ({ topicId }) =>
        useRealtimeTimeline({
          topicId,
          mode: 'realtime',
          onFallbackToStandard: vi.fn(),
        }),
      {
        wrapper: buildWrapper(queryClient),
        initialProps: { topicId: 'topic-1' },
      },
    );

    const topic1Payload: NostrEventPayload = {
      id: 'topic1-parent-2',
      author: 'author-topic1',
      content: 'topic1 pending delta',
      created_at: 1_700_000_060,
      kind: 30078,
      tags: [
        ['t', 'topic-1'],
        ['thread', 'topic-1/threads/topic1-thread-2'],
        ['thread_uuid', 'topic1-thread-2'],
      ],
    };

    const topic2Payload: NostrEventPayload = {
      id: 'topic2-parent-2',
      author: 'author-topic2',
      content: 'topic2 new delta',
      created_at: 1_700_000_120,
      kind: 30078,
      tags: [
        ['t', 'topic-2'],
        ['thread', 'topic-2/threads/topic2-thread-2'],
        ['thread_uuid', 'topic2-thread-2'],
      ],
    };

    act(() => {
      dispatchRealtimeDelta({
        source: 'nostr',
        payload: topic1Payload,
        receivedAt: Date.now(),
      });
    });

    rerender({ topicId: 'topic-2' });

    act(() => {
      dispatchRealtimeDelta({
        source: 'nostr',
        payload: topic2Payload,
        receivedAt: Date.now(),
      });
      vi.advanceTimersByTime(750);
    });

    const topic2Entries = queryClient.getQueryData<TopicTimelineEntry[]>([
      'topicTimeline',
      'topic-2',
    ]);
    expect(topic2Entries).toHaveLength(2);
    expect(topic2Entries?.some((entry) => entry.threadUuid === 'topic2-thread-2')).toBe(true);
  });

  it('realtime 中に接続が切れたら standard へフォールバックする', () => {
    const queryClient = buildQueryClient();
    const onFallbackToStandard = vi.fn();

    renderHook(
      () =>
        useRealtimeTimeline({
          topicId: 'topic-1',
          mode: 'realtime',
          onFallbackToStandard,
        }),
      { wrapper: buildWrapper(queryClient) },
    );

    act(() => {
      window.dispatchEvent(new Event('offline'));
    });

    expect(onFallbackToStandard).toHaveBeenCalledTimes(1);
  });
});
