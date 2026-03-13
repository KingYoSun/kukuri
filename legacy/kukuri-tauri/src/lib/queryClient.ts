import { QueryClient } from '@tanstack/react-query';
import { useOfflineStore } from '@/stores/offlineStore';
import { toast } from 'sonner';

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5分
      gcTime: 1000 * 60 * 10, // 10分（以前のcacheTime）
      retry: (failureCount, _error) => {
        // オフラインの場合はリトライしない
        const isOnline = useOfflineStore.getState().isOnline;
        if (!isOnline) return false;

        // オンラインの場合は3回までリトライ
        return failureCount < 3;
      },
      refetchOnWindowFocus: false,
      refetchOnReconnect: 'always', // ネットワーク復帰時は必ず再フェッチ
      // オフライン時はネットワークリクエストを抑制
      networkMode: 'offlineFirst',
    },
    mutations: {
      retry: (failureCount, _error) => {
        // オフラインの場合はリトライしない
        const isOnline = useOfflineStore.getState().isOnline;
        if (!isOnline) return false;

        // オンラインの場合は1回までリトライ
        return failureCount < 1;
      },
      // 楽観的更新のためのデフォルト設定
      networkMode: 'offlineFirst',
    },
  },
});

// オンライン/オフライン状態の変化を監視してクエリを再実行
if (typeof window !== 'undefined') {
  window.addEventListener('online', () => {
    const offlineStore = useOfflineStore.getState();
    offlineStore.setOnlineStatus(true);

    // オンラインになったら保留中のクエリを再実行
    queryClient.resumePausedMutations();
    queryClient.invalidateQueries();

    toast.success('オンラインに復帰しました');
  });

  window.addEventListener('offline', () => {
    const offlineStore = useOfflineStore.getState();
    offlineStore.setOnlineStatus(false);

    // オフラインになったらクエリを一時停止
    queryClient.cancelQueries();

    toast.info('オフラインモードで動作中です');
  });
}

// カスタムキャッシュ管理ユーティリティ
export const cacheUtils = {
  /**
   * 特定のクエリキーのキャッシュをプリフェッチ
   */
  prefetchQuery: async (queryKey: string[], queryFn: () => Promise<any>) => {
    await queryClient.prefetchQuery({
      queryKey,
      queryFn,
      staleTime: 1000 * 60 * 10, // 10分間新鮮
    });
  },

  /**
   * オフライン用にキャッシュを永続化
   */
  persistCache: (queryKey: string[], data: any) => {
    queryClient.setQueryData(queryKey, data, {
      updatedAt: Date.now(),
    });
  },

  /**
   * キャッシュのクリア（選択的）
   */
  clearCache: (queryKey?: string[]) => {
    if (queryKey) {
      queryClient.removeQueries({ queryKey });
    } else {
      queryClient.clear();
    }
  },

  /**
   * キャッシュの有効性を確認
   */
  isCacheValid: (queryKey: string[]): boolean => {
    const state = queryClient.getQueryState(queryKey);
    if (!state) return false;

    const now = Date.now();
    const dataUpdatedAt = state.dataUpdatedAt;
    const staleTime = 1000 * 60 * 5; // 5分

    return now - dataUpdatedAt < staleTime;
  },

  /**
   * オフライン時のキャッシュ最適化
   */
  optimizeForOffline: () => {
    // 重要なデータのキャッシュ時間を延長
    const importantQueries = [
      ['topics'],
      ['posts'],
      ['timeline'],
      ['topicTimeline'],
      ['bookmarks'],
    ];

    importantQueries.forEach((queryKey) => {
      const data = queryClient.getQueryData(queryKey);
      if (data) {
        queryClient.setQueryData(queryKey, data, {
          updatedAt: Date.now(),
        });
      }
    });
  },
};
