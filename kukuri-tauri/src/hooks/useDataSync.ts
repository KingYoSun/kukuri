import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useAuthStore } from '@/stores/authStore';

/**
 * データ同期フック
 * React QueryとZustandストアの間でデータを同期し、
 * リアルタイム更新を実現します
 */
export function useDataSync() {
  const queryClient = useQueryClient();
  const { isAuthenticated } = useAuthStore();

  useEffect(() => {
    if (!isAuthenticated) {
      return;
    }

    // 投稿ストアの変更を監視
    const unsubscribePosts = usePostStore.subscribe((state) => {
      // ストアが変更されたらReact Queryのキャッシュも更新
      const posts = state.posts;
      queryClient.setQueryData(['posts'], () => {
        return {
          pages: [
            {
              posts: Array.from(posts.values()).sort(
                (a, b) => b.created_at - a.created_at
              ),
            },
          ],
          pageParams: [],
        };
      });
    });

    // トピックストアの変更を監視
    const unsubscribeTopics = useTopicStore.subscribe((state) => {
      // ストアが変更されたらReact Queryのキャッシュも更新
      const topics = state.topics;
      queryClient.setQueryData(['topics'], () => {
        return Array.from(topics.values());
      });
    });

    // クリーンアップ
    return () => {
      unsubscribePosts();
      unsubscribeTopics();
    };
  }, [isAuthenticated, queryClient]);

  // 定期的なデータ同期（フォールバック）
  useEffect(() => {
    if (!isAuthenticated) {
      return;
    }

    // 5分ごとに全データを再取得（念のため）
    const interval = setInterval(() => {
      // React Queryのstaleデータを再取得
      queryClient.refetchQueries({
        queryKey: ['posts'],
        type: 'active',
        stale: true,
      });
      queryClient.refetchQueries({
        queryKey: ['topics'],
        type: 'active',
        stale: true,
      });
    }, 5 * 60 * 1000); // 5分

    return () => {
      clearInterval(interval);
    };
  }, [isAuthenticated, queryClient]);

  // オフライン/オンライン状態の監視
  useEffect(() => {
    const handleOnline = () => {
      // オンラインに復帰したら最新データを取得
      queryClient.refetchQueries();
    };

    window.addEventListener('online', handleOnline);

    return () => {
      window.removeEventListener('online', handleOnline);
    };
  }, [queryClient]);
}