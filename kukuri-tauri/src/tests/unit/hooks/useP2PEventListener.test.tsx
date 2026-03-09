import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';

vi.unmock('@/hooks/useP2PEventListener');

const { listeners, listenMock } = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  listenMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock.mockImplementation(
    async (event: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(event, handler);
      return () => {
        listeners.delete(event);
      };
    },
  ),
}));

vi.mock('@/lib/utils/tauriEnvironment', () => ({
  isTauriRuntime: () => true,
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

describe('useP2PEventListener', () => {
  beforeEach(() => {
    listeners.clear();
    vi.clearAllMocks();
    vi.spyOn(TauriApi, 'getUserProfileByPubkey').mockResolvedValue(null);
    vi.spyOn(TauriApi, 'getUserProfile').mockResolvedValue(null);
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
      expect(listeners.has('p2p://message/raw')).toBe(true);
    });

    const rawHandler = listeners.get('p2p://message/raw');
    expect(rawHandler).toBeDefined();

    await act(async () => {
      rawHandler?.({
        payload: {
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
        },
      });
      await flushPromises();
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
});
