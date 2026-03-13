import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';

vi.unmock('@/hooks/useP2PEventListener');

const { listeners, listenMock } = vi.hoisted(() => ({
  listeners: new Map<string, Array<(event: { payload: unknown }) => void>>(),
  listenMock: vi.fn(),
}));

const { resolveAuthorProfileWithRelayFallbackMock } = vi.hoisted(() => ({
  resolveAuthorProfileWithRelayFallbackMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock.mockImplementation(
    async (event: string, handler: (event: { payload: unknown }) => void) => {
      const registered = listeners.get(event) ?? [];
      listeners.set(event, [...registered, handler]);
      return () => {
        const current = listeners.get(event) ?? [];
        const next = current.filter((candidate) => candidate !== handler);
        if (next.length === 0) {
          listeners.delete(event);
          return;
        }
        listeners.set(event, next);
      };
    },
  ),
}));

vi.mock('@/lib/utils/tauriEnvironment', () => ({
  isTauriRuntime: () => true,
}));

vi.mock('@/lib/profile/authorProfileResolver', () => ({
  resolveAuthorProfileWithRelayFallback: resolveAuthorProfileWithRelayFallbackMock,
}));

import { useP2PEventListener } from '@/hooks/useP2PEventListener';
import type { TopicTimelineEntry } from '@/hooks/usePosts';
import { TauriApi } from '@/lib/api/tauri';
import { usePostStore } from '@/stores/postStore';
import type { Post } from '@/stores/types';

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

const topicId = 'topic-profile-sync';
const threadUuid = 'thread-profile-sync';
const authorId = 'a'.repeat(64);
const metadataEventId = 'b'.repeat(64);
const metadataSignature = 'c'.repeat(128);

const basePost: Post = {
  id: 'post-profile-sync',
  eventId: 'post-profile-sync',
  content: 'base post',
  author: {
    id: authorId,
    pubkey: authorId,
    npub: authorId,
    name: 'Old Name',
    displayName: 'Old Display',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
    publicProfile: true,
    showOnlineStatus: false,
  },
  topicId,
  threadNamespace: `${topicId}/threads/${threadUuid}`,
  threadUuid,
  threadRootEventId: 'post-profile-sync',
  threadParentEventId: null,
  created_at: 1_700_000_000,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
  replyCount: 0,
  isSynced: true,
};

const baseEntry: TopicTimelineEntry = {
  threadUuid,
  parentPost: basePost,
  firstReply: null,
  replyCount: 0,
  lastActivityAt: 1_700_000_000,
};

const flushPromises = async () => {
  await Promise.resolve();
  await Promise.resolve();
};

const emitToListeners = async (event: string, payload: unknown) => {
  const handlers = listeners.get(event) ?? [];
  for (const handler of handlers) {
    await act(async () => {
      handler({ payload });
      await flushPromises();
    });
  }
};

describe('useP2PEventListener', () => {
  beforeEach(() => {
    listeners.clear();
    vi.clearAllMocks();
    vi.spyOn(TauriApi, 'getUserProfileByPubkey').mockResolvedValue(null);
    vi.spyOn(TauriApi, 'getUserProfile').mockResolvedValue(null);
    resolveAuthorProfileWithRelayFallbackMock.mockResolvedValue(null);
    usePostStore.getState().setPosts([basePost]);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    listeners.clear();
  });

  it('metadata(kind=0) の P2P raw event で author metadata を即時反映する', async () => {
    const queryClient = buildQueryClient();
    queryClient.setQueryData<Post[]>(['timeline'], [basePost]);
    queryClient.setQueryData<Post[]>(['posts', topicId], [basePost]);
    queryClient.setQueryData<Post[]>(['threadPosts', topicId, threadUuid], [basePost]);
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicTimeline', topicId], [baseEntry]);
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicThreads', topicId], [baseEntry]);

    renderHook(() => useP2PEventListener(), {
      wrapper: buildWrapper(queryClient),
    });

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalled();
      expect((listeners.get('p2p://message/raw') ?? []).length).toBeGreaterThan(0);
    });

    await emitToListeners('p2p://message/raw', {
      topic_id: topicId,
      payload: JSON.stringify({
        id: metadataEventId,
        pubkey: authorId,
        content: JSON.stringify({
          name: 'Updated Name',
          display_name: 'Updated Display',
          about: 'Updated bio',
          picture: 'https://example.com/avatar.png',
          nip05: 'updated@example.com',
          kukuri_privacy: {
            public_profile: true,
            show_online_status: false,
          },
        }),
        sig: metadataSignature,
        kind: 0,
        tags: [['t', topicId]],
        created_at: 1_700_000_100,
      }),
      timestamp: 1_700_000_100,
    });

    const storedPost = usePostStore.getState().posts.get(basePost.id);
    expect(storedPost?.author.displayName).toBe('Updated Display');
    expect(storedPost?.author.picture).toBe('https://example.com/avatar.png');

    const timeline = queryClient.getQueryData<Post[]>(['timeline']);
    expect(timeline?.[0].author.displayName).toBe('Updated Display');
    expect(timeline?.[0].author.nip05).toBe('updated@example.com');

    const threadPosts = queryClient.getQueryData<Post[]>(['threadPosts', topicId, threadUuid]);
    expect(threadPosts?.[0].author.displayName).toBe('Updated Display');

    const topicTimeline = queryClient.getQueryData<TopicTimelineEntry[]>([
      'topicTimeline',
      topicId,
    ]);
    expect(topicTimeline?.[0].parentPost.author.displayName).toBe('Updated Display');
    expect(topicTimeline?.[0].parentPost.author.about).toBe('Updated bio');

    const topicThreads = queryClient.getQueryData<TopicTimelineEntry[]>(['topicThreads', topicId]);
    expect(topicThreads?.[0].parentPost.author.picture).toBe('https://example.com/avatar.png');
  });

  it('複数 mount されても listener を共有し、topic post を重複登録しない', async () => {
    const queryClient = buildQueryClient();
    queryClient.setQueryData<Post[]>(['timeline'], [basePost]);
    queryClient.setQueryData<Post[]>(['posts', topicId], [basePost]);
    queryClient.setQueryData<Post[]>(['threadPosts', topicId, threadUuid], [basePost]);
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicTimeline', topicId], [baseEntry]);
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicThreads', topicId], [baseEntry]);

    renderHook(() => useP2PEventListener(), {
      wrapper: buildWrapper(queryClient),
    });
    renderHook(() => useP2PEventListener(), {
      wrapper: buildWrapper(queryClient),
    });

    await waitFor(() => {
      expect(listenMock).toHaveBeenCalledTimes(4);
      expect((listeners.get('p2p://message/raw') ?? []).length).toBe(1);
    });

    const propagatedEventId = 'd'.repeat(64);
    await emitToListeners('p2p://message/raw', {
      topic_id: topicId,
      payload: JSON.stringify({
        id: propagatedEventId,
        pubkey: authorId,
        content: 'propagated topic post',
        sig: 'e'.repeat(128),
        kind: 30078,
        tags: [
          ['t', topicId],
          ['thread', `${topicId}/threads/${threadUuid}`],
          ['thread_uuid', threadUuid],
        ],
        created_at: 1_700_000_200,
      }),
      timestamp: 1_700_000_200,
    });

    const state = usePostStore.getState();
    expect(state.posts.has(propagatedEventId)).toBe(true);
    expect(state.postsByTopic.get(topicId)).toEqual([basePost.id, propagatedEventId]);
  });

  it('topic post 受信時に relay pull で author metadata を補完する', async () => {
    const queryClient = buildQueryClient();
    queryClient.setQueryData<Post[]>(['timeline'], [basePost]);
    queryClient.setQueryData<Post[]>(['posts', topicId], [basePost]);
    queryClient.setQueryData<Post[]>(['threadPosts', topicId, threadUuid], [basePost]);
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicTimeline', topicId], [baseEntry]);
    queryClient.setQueryData<TopicTimelineEntry[]>(['topicThreads', topicId], [baseEntry]);

    resolveAuthorProfileWithRelayFallbackMock.mockResolvedValue({
      id: authorId,
      pubkey: authorId,
      npub: 'npub1relayauthor',
      name: 'relay-author',
      displayName: 'Relay Author',
      about: 'relay resolved profile',
      picture:
        'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgwJ/l7hR9QAAAABJRU5ErkJggg==',
      nip05: 'relay@example.com',
      avatar: null,
      publicProfile: true,
      showOnlineStatus: false,
    });

    renderHook(() => useP2PEventListener(), {
      wrapper: buildWrapper(queryClient),
    });

    await waitFor(() => {
      expect((listeners.get('p2p://message/raw') ?? []).length).toBeGreaterThan(0);
    });

    const propagatedEventId = 'f'.repeat(64);
    await emitToListeners('p2p://message/raw', {
      topic_id: topicId,
      payload: JSON.stringify({
        id: propagatedEventId,
        pubkey: authorId,
        content: 'relay hydrated post',
        sig: 'd'.repeat(128),
        kind: 30078,
        tags: [
          ['t', topicId],
          ['thread', `${topicId}/threads/${threadUuid}`],
          ['thread_uuid', threadUuid],
        ],
        created_at: 1_700_000_300,
      }),
      timestamp: 1_700_000_300,
    });

    await waitFor(() => {
      const storedPost = usePostStore.getState().posts.get(propagatedEventId);
      expect(storedPost?.author.displayName).toBe('Relay Author');
      expect(storedPost?.author.picture).toContain('data:image/png;base64,');
    });

    const timeline = queryClient.getQueryData<Post[]>(['timeline']);
    expect(timeline?.some((post) => post.author.displayName === 'Relay Author')).toBe(true);
  });
});
