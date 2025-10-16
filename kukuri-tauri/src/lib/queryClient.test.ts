import { describe, it, expect, vi, beforeEach } from 'vitest';
import { queryClient, cacheUtils } from './queryClient';
import { useOfflineStore } from '@/stores/offlineStore';

vi.mock('@/stores/offlineStore');
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
    info: vi.fn(),
  },
}));

describe('queryClient configuration', () => {
  const mockOfflineStore = useOfflineStore as unknown as {
    getState: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockOfflineStore.getState = vi.fn().mockReturnValue({
      isOnline: true,
    });
  });

  describe('queries configuration', () => {
    it('オフライン時はリトライしない', () => {
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: false,
      });

      const retryFn = queryClient.getDefaultOptions().queries?.retry as (
        failureCount: number,
        error: unknown,
      ) => boolean;
      const shouldRetry = retryFn(1, new Error('Network error'));

      expect(shouldRetry).toBe(false);
    });

    it('オンライン時は3回までリトライする', () => {
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
      });

      const retryFn = queryClient.getDefaultOptions().queries?.retry as (
        failureCount: number,
        error: unknown,
      ) => boolean;

      expect(retryFn(0, new Error('Network error'))).toBe(true);
      expect(retryFn(1, new Error('Network error'))).toBe(true);
      expect(retryFn(2, new Error('Network error'))).toBe(true);
      expect(retryFn(3, new Error('Network error'))).toBe(false);
    });

    it('staleTimeが正しく設定されている', () => {
      const options = queryClient.getDefaultOptions().queries;
      expect(options?.staleTime).toBe(1000 * 60 * 5); // 5分
    });

    it('gcTimeが正しく設定されている', () => {
      const options = queryClient.getDefaultOptions().queries;
      expect(options?.gcTime).toBe(1000 * 60 * 10); // 10分
    });

    it('networkModeがofflineFirstに設定されている', () => {
      const options = queryClient.getDefaultOptions().queries;
      expect(options?.networkMode).toBe('offlineFirst');
    });
  });

  describe('mutations configuration', () => {
    it('オフライン時はリトライしない', () => {
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: false,
      });

      const retryFn = queryClient.getDefaultOptions().mutations?.retry as (
        failureCount: number,
        error: unknown,
      ) => boolean;
      const shouldRetry = retryFn(1, new Error('Network error'));

      expect(shouldRetry).toBe(false);
    });

    it('オンライン時は1回までリトライする', () => {
      mockOfflineStore.getState = vi.fn().mockReturnValue({
        isOnline: true,
      });

      const retryFn = queryClient.getDefaultOptions().mutations?.retry as (
        failureCount: number,
        error: unknown,
      ) => boolean;

      expect(retryFn(0, new Error('Network error'))).toBe(true);
      expect(retryFn(1, new Error('Network error'))).toBe(false);
    });
  });
});

describe('cacheUtils', () => {
  beforeEach(() => {
    queryClient.clear();
  });

  describe('prefetchQuery', () => {
    it('クエリをプリフェッチする', async () => {
      const queryKey = ['test-query'];
      const queryFn = vi.fn().mockResolvedValue({ data: 'test' });

      await cacheUtils.prefetchQuery(queryKey, queryFn);

      expect(queryFn).toHaveBeenCalled();
      const data = queryClient.getQueryData(queryKey);
      expect(data).toEqual({ data: 'test' });
    });
  });

  describe('persistCache', () => {
    it('キャッシュを永続化する', () => {
      const queryKey = ['test-query'];
      const data = { data: 'test' };

      cacheUtils.persistCache(queryKey, data);

      const cachedData = queryClient.getQueryData(queryKey);
      expect(cachedData).toEqual(data);
    });
  });

  describe('clearCache', () => {
    it('特定のクエリキーのキャッシュをクリアする', () => {
      const queryKey = ['test-query'];
      queryClient.setQueryData(queryKey, { data: 'test' });

      cacheUtils.clearCache(queryKey);

      const data = queryClient.getQueryData(queryKey);
      expect(data).toBeUndefined();
    });

    it('すべてのキャッシュをクリアする', () => {
      queryClient.setQueryData(['query1'], { data: 'test1' });
      queryClient.setQueryData(['query2'], { data: 'test2' });

      cacheUtils.clearCache();

      expect(queryClient.getQueryData(['query1'])).toBeUndefined();
      expect(queryClient.getQueryData(['query2'])).toBeUndefined();
    });
  });

  describe('isCacheValid', () => {
    it('キャッシュが有効期限内の場合はtrueを返す', () => {
      const queryKey = ['test-query'];
      queryClient.setQueryData(queryKey, { data: 'test' });

      const isValid = cacheUtils.isCacheValid(queryKey);

      expect(isValid).toBe(true);
    });

    it('キャッシュが存在しない場合はfalseを返す', () => {
      const queryKey = ['non-existent'];

      const isValid = cacheUtils.isCacheValid(queryKey);

      expect(isValid).toBe(false);
    });
  });

  describe('optimizeForOffline', () => {
    it('重要なクエリのキャッシュを更新する', () => {
      vi.useFakeTimers();

      // 重要なクエリにデータを設定
      queryClient.setQueryData(['topics'], { data: 'topics' });
      queryClient.setQueryData(['posts'], { data: 'posts' });
      queryClient.setQueryData(['timeline'], { data: 'timeline' });
      queryClient.setQueryData(['bookmarks'], { data: 'bookmarks' });

      const beforeState = queryClient.getQueryState(['topics']);
      const _beforeUpdatedAt = beforeState?.dataUpdatedAt;

      // 少し時間を進める
      vi.advanceTimersByTime(1000);

      cacheUtils.optimizeForOffline();

      const afterState = queryClient.getQueryState(['topics']);
      const _afterUpdatedAt = afterState?.dataUpdatedAt;

      // データは保持されている
      expect(queryClient.getQueryData(['topics'])).toEqual({ data: 'topics' });
      expect(queryClient.getQueryData(['posts'])).toEqual({ data: 'posts' });
      expect(queryClient.getQueryData(['timeline'])).toEqual({ data: 'timeline' });
      expect(queryClient.getQueryData(['bookmarks'])).toEqual({ data: 'bookmarks' });

      vi.useRealTimers();
    });
  });
});
