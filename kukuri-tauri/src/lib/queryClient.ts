import { QueryClient } from '@tanstack/react-query';
import { useOfflineStore } from '@/stores/offlineStore';

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5分
      gcTime: 1000 * 60 * 10, // 10分（以前のcacheTime）
      retry: (failureCount, error) => {
        // オフラインの場合はリトライしない
        const isOnline = useOfflineStore.getState().isOnline;
        if (!isOnline) return false;
        
        // オンラインの場合は3回までリトライ
        return failureCount < 3;
      },
      refetchOnWindowFocus: false,
      // オフライン時はネットワークリクエストを抑制
      networkMode: 'offlineFirst',
    },
    mutations: {
      retry: (failureCount, error) => {
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
    // オンラインになったら保留中のクエリを再実行
    queryClient.resumePausedMutations();
    queryClient.invalidateQueries();
  });

  window.addEventListener('offline', () => {
    // オフラインになったらクエリを一時停止
    queryClient.cancelQueries();
  });
}
