import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactNode } from 'react';

import { useDeletePost } from '@/hooks/usePosts';
import type { Post } from '@/stores';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

const {
  mockDeletePostRemote,
  mockUpdateTopicPostCount,
  offlineState,
  postStoreState,
  usePostStoreMock,
  useTopicStoreMock,
  useOfflineStoreMock,
  invalidatePostCachesMock,
} = vi.hoisted(() => {
  const mockDeletePostRemote = vi.fn();
  const mockUpdateTopicPostCount = vi.fn();
  const offlineState = { isOnline: true };

  const postStoreState = {
    deletePostRemote: mockDeletePostRemote,
    posts: new Map<string, Post>(),
  };

  const usePostStoreMock = vi.fn((selector?: (state: typeof postStoreState) => unknown) =>
    selector ? selector(postStoreState) : postStoreState,
  ) as unknown as ((selector?: (state: typeof postStoreState) => unknown) => unknown) & {
    getState: () => typeof postStoreState;
  };
  usePostStoreMock.getState = () => postStoreState;

  const useTopicStoreMock = vi.fn(
    (selector?: (state: { updateTopicPostCount: typeof mockUpdateTopicPostCount }) => unknown) => {
      const state = {
        updateTopicPostCount: mockUpdateTopicPostCount,
      };
      return selector ? selector(state) : state;
    },
  );

  const useOfflineStoreMock = vi.fn((selector?: (state: { isOnline: boolean }) => unknown) => {
    const state = {
      isOnline: offlineState.isOnline,
    };
    return selector ? selector(state) : state;
  });

  const invalidatePostCachesMock = vi.fn();

  return {
    mockDeletePostRemote,
    mockUpdateTopicPostCount,
    offlineState,
    postStoreState,
    usePostStoreMock,
    useTopicStoreMock,
    useOfflineStoreMock,
    invalidatePostCachesMock,
  };
});

vi.mock('@/stores', () => ({
  usePostStore: usePostStoreMock,
}));

vi.mock('@/stores/topicStore', () => ({
  useTopicStore: (
    selector?: (state: { updateTopicPostCount: typeof mockUpdateTopicPostCount }) => unknown,
  ) => useTopicStoreMock(selector),
}));

vi.mock('@/stores/offlineStore', () => ({
  useOfflineStore: (selector?: (state: { isOnline: boolean }) => unknown) =>
    useOfflineStoreMock(selector),
}));

vi.mock('@/lib/posts/cacheUtils', () => ({
  invalidatePostCaches: (...args: unknown[]) => invalidatePostCachesMock(...args),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    info: vi.fn(),
    log: vi.fn(),
  },
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

const samplePost: Post = {
  id: 'post-1',
  content: 'draft',
  author: {
    id: 'user-1',
    pubkey: 'pubkey-1',
    npub: 'npub1samp1e',
    name: 'Tester',
    displayName: 'Tester',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  },
  topicId: 'topic-1',
  created_at: 1_695_000_000,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
  isSynced: true,
};

describe('useDeletePost', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    offlineState.isOnline = true;
    postStoreState.posts = new Map([[samplePost.id, samplePost]]);
  });

  it('オンライン時に投稿削除とキャッシュ更新を行う', async () => {
    const wrapper = createWrapper();
    mockDeletePostRemote.mockResolvedValue(undefined);
    const { result } = renderHook(() => useDeletePost(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync(samplePost);
    });

    expect(mockDeletePostRemote).toHaveBeenCalledWith({
      id: samplePost.id,
      topicId: samplePost.topicId,
      authorPubkey: samplePost.author.pubkey,
    });
    expect(mockUpdateTopicPostCount).toHaveBeenCalledWith(samplePost.topicId, -1);
    expect(invalidatePostCachesMock).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        id: samplePost.id,
        topicId: samplePost.topicId,
        authorPubkey: samplePost.author.pubkey,
      }),
    );
    expect(toast.success).toHaveBeenCalledWith('投稿を削除しました');
  });

  it('オフライン時はキュー登録メッセージを表示する', async () => {
    const wrapper = createWrapper();
    offlineState.isOnline = false;
    mockDeletePostRemote.mockResolvedValue(undefined);

    const { result } = renderHook(() => useDeletePost(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync(samplePost);
    });

    expect(toast.success).toHaveBeenCalledWith('削除は接続復旧後に反映されます');
    expect(errorHandler.info).toHaveBeenCalledWith('Post.delete_offline_enqueued', 'useDeletePost');
  });

  it('失敗時はログとトーストを表示する', async () => {
    const wrapper = createWrapper();
    const failure = new Error('network');
    mockDeletePostRemote.mockRejectedValue(failure);

    const { result } = renderHook(() => useDeletePost(), { wrapper });

    await expect(result.current.mutateAsync(samplePost)).rejects.toThrowError();

    expect(errorHandler.log).toHaveBeenCalledWith(
      'Post.delete_failed',
      failure,
      expect.objectContaining({
        context: 'useDeletePost',
        metadata: expect.objectContaining({
          postId: samplePost.id,
        }),
      }),
    );
    expect(toast.error).toHaveBeenCalledWith('投稿の削除に失敗しました');
  });
  it('manualRetryDelete はキャッシュ内の投稿を再利用する', async () => {
    const wrapper = createWrapper();
    mockDeletePostRemote.mockResolvedValue(undefined);
    const { result } = renderHook(() => useDeletePost(), { wrapper });

    await act(async () => {
      await result.current.manualRetryDelete({ postId: samplePost.id });
    });

    expect(mockDeletePostRemote).toHaveBeenCalledWith({
      id: samplePost.id,
      topicId: samplePost.topicId,
      authorPubkey: samplePost.author.pubkey,
    });
  });

  it('manualRetryDelete はメタデータのみでも再送できる', async () => {
    const wrapper = createWrapper();
    postStoreState.posts.clear();
    mockDeletePostRemote.mockResolvedValue(undefined);
    const { result } = renderHook(() => useDeletePost(), { wrapper });

    await act(async () => {
      await result.current.manualRetryDelete({
        postId: samplePost.id,
        topicId: 'fallback-topic',
        authorPubkey: 'author-fallback',
      });
    });

    expect(mockDeletePostRemote).toHaveBeenCalledWith({
      id: samplePost.id,
      topicId: 'fallback-topic',
      authorPubkey: 'author-fallback',
    });
    expect(invalidatePostCachesMock).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        id: samplePost.id,
        topicId: 'fallback-topic',
        authorPubkey: 'author-fallback',
      }),
    );
  });
});
