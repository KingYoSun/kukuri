import { describe, it, expect, vi, beforeEach, afterEach, type MockedFunction } from 'vitest';
import { renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactNode } from 'react';
import { useNostrEvents } from './useNostrEvents';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import type { NostrEventPayload } from '@/types/nostr';

// Tauri APIのモック
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

// ストアのモック
vi.mock('@/stores/postStore');
vi.mock('@/stores/topicStore');

// error-handlerのモック
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: vi.fn(),
}));

describe('useNostrEvents', () => {
  let queryClient: QueryClient;
  let mockListen: MockedFunction<typeof import('@tauri-apps/api/event').listen>;
  let mockUnlisten: vi.Mock;
  let listeners: Map<string, (event: { payload: NostrEventPayload }) => void>;

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  beforeEach(async () => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    listeners = new Map();
    mockUnlisten = vi.fn();

    const { listen } = await import('@tauri-apps/api/event');
    mockListen = listen as MockedFunction<typeof import('@tauri-apps/api/event').listen>;

    mockListen.mockImplementation((event: string, handler: (event: { payload: NostrEventPayload }) => void) => {
      listeners.set(event, handler);
      return Promise.resolve(mockUnlisten);
    });

    // ストアのモックを設定
    const mockIncrementLikes = vi.fn();
    const mockUpdatePostLikes = vi.fn();
    const mockUpdateTopicPostCount = vi.fn();

    vi.mocked(usePostStore).mockReturnValue({
      incrementLikes: mockIncrementLikes,
      updatePostLikes: mockUpdatePostLikes,
    } as Partial<ReturnType<typeof usePostStore>> as ReturnType<typeof usePostStore>);

    vi.mocked(useTopicStore).mockReturnValue({
      updateTopicPostCount: mockUpdateTopicPostCount,
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);
  });

  afterEach(() => {
    vi.clearAllMocks();
    listeners.clear();
  });

  it('should setup listener on mount and cleanup on unmount', async () => {
    const { unmount } = renderHook(() => useNostrEvents(), { wrapper });

    // リスナーが設定されたことを確認（非同期なので待つ）
    await vi.waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith('nostr://event', expect.any(Function));
    });

    // アンマウント時にクリーンアップされることを確認
    unmount();

    // クリーンアップ関数が呼ばれるのを待つ
    await vi.waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });

  it('should handle post events (kind: 1)', async () => {
    renderHook(() => useNostrEvents(), { wrapper });

    const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries');
    const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent');

    const handler = listeners.get('nostr://event');
    const payload: NostrEventPayload = {
      id: 'test-id',
      author: 'test-author',
      content: 'Test post',
      created_at: Date.now(),
      kind: 1,
      tags: [],
    };

    handler?.({ payload });

    // キャッシュが無効化されることを確認
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['posts'] });

    // リアルタイム更新イベントが発火されることを確認
    expect(dispatchEventSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'realtime-update',
      }),
    );
  });

  it('should handle topic post events (kind: 30078)', async () => {
    const { updateTopicPostCount } = useTopicStore();
    renderHook(() => useNostrEvents(), { wrapper });

    const handler = listeners.get('nostr://event');
    const payload: NostrEventPayload = {
      id: 'test-id',
      author: 'test-author',
      content: 'Test topic post',
      created_at: Date.now(),
      kind: 30078,
      tags: [['t', 'topic-123']],
    };

    handler?.({ payload });

    // トピックの投稿数が更新されることを確認
    expect(updateTopicPostCount).toHaveBeenCalledWith('topic-123', 1);
  });

  it('should handle reaction events (kind: 7)', async () => {
    const { incrementLikes } = usePostStore();
    renderHook(() => useNostrEvents(), { wrapper });

    const setQueryDataSpy = vi.spyOn(queryClient, 'setQueryData');

    const handler = listeners.get('nostr://event');
    const payload: NostrEventPayload = {
      id: 'reaction-id',
      author: 'test-author',
      content: '+',
      created_at: Date.now(),
      kind: 7,
      tags: [['e', 'post-123']],
    };

    handler?.({ payload });

    // いいね数が増加することを確認
    expect(incrementLikes).toHaveBeenCalledWith('post-123');

    // React Queryのキャッシュが更新されることを確認
    expect(setQueryDataSpy).toHaveBeenCalled();
  });

  it('should handle topic events (kind: 30030)', async () => {
    renderHook(() => useNostrEvents(), { wrapper });

    const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const handler = listeners.get('nostr://event');
    const payload: NostrEventPayload = {
      id: 'topic-id',
      author: 'test-author',
      content: 'Topic metadata',
      created_at: Date.now(),
      kind: 30030,
      tags: [],
    };

    handler?.({ payload });

    // トピックのキャッシュが無効化されることを確認
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['topics'] });
  });

  it('should handle delete events (kind: 5)', async () => {
    renderHook(() => useNostrEvents(), { wrapper });

    const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries');

    const handler = listeners.get('nostr://event');
    const payload: NostrEventPayload = {
      id: 'delete-id',
      author: 'test-author',
      content: '',
      created_at: Date.now(),
      kind: 5,
      tags: [
        ['e', 'event-1'],
        ['e', 'event-2'],
      ],
    };

    handler?.({ payload });

    // 投稿とトピックのキャッシュが無効化されることを確認
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['posts'] });
    expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['topics'] });
  });

  it('should ignore unknown event kinds', async () => {
    renderHook(() => useNostrEvents(), { wrapper });

    const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries');
    const { incrementLikes } = usePostStore();

    const handler = listeners.get('nostr://event');
    const payload: NostrEventPayload = {
      id: 'unknown-id',
      author: 'test-author',
      content: 'Unknown event',
      created_at: Date.now(),
      kind: 9999, // 未知のイベント種別
      tags: [],
    };

    handler?.({ payload });

    // 何も処理されないことを確認
    expect(invalidateQueriesSpy).not.toHaveBeenCalled();
    expect(incrementLikes).not.toHaveBeenCalled();
  });
});
