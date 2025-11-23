import { useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTopicStore } from '@/stores';
import type { Topic } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';

// トピック一覧を取得するフック
export const useTopics = () => {
  const { setTopics } = useTopicStore();
  const refreshPendingTopics = useTopicStore((state) => state.refreshPendingTopics);

  useEffect(() => {
    if (typeof refreshPendingTopics !== 'function') {
      return;
    }
    void refreshPendingTopics();
  }, [refreshPendingTopics]);

  return useQuery({
    queryKey: ['topics'],
    queryFn: async () => {
      const apiTopics = await TauriApi.getTopics();

      // 各トピックの統計情報を並列で取得
      const topicsWithStats = await Promise.all(
        apiTopics.map(async (topic) => {
          try {
            const stats = await TauriApi.getTopicStats(topic.id);
            return {
              id: topic.id,
              name: topic.name,
              description: topic.description ?? '',
              tags: [], // APIにタグ情報がない場合は空配列
              memberCount: stats.member_count,
              postCount: stats.post_count,
              lastActive: topic.updated_at,
              isActive: true,
              createdAt: new Date(topic.created_at * 1000),
              visibility: topic.visibility ?? 'public',
              isJoined: Boolean(topic.is_joined),
            } as Topic;
          } catch (error) {
            // 統計情報の取得に失敗した場合はデフォルト値を使用
            errorHandler.log(`Failed to get stats for topic ${topic.id}`, error, {
              context: 'useTopics.getTopicStats',
            });
            return {
              id: topic.id,
              name: topic.name,
              description: topic.description ?? '',
              tags: [],
              memberCount: topic.member_count ?? 0,
              postCount: topic.post_count ?? 0,
              lastActive: topic.updated_at,
              isActive: true,
              createdAt: new Date(topic.created_at * 1000),
              visibility: topic.visibility ?? 'public',
              isJoined: Boolean(topic.is_joined),
            } as Topic;
          }
        }),
      );

      setTopics(topicsWithStats);
      return topicsWithStats;
    },
    refetchInterval: 30000, // 30秒ごとに更新
  });
};

// 単一トピックを取得するフック
export const useTopic = (topicId: string) => {
  const { topics } = useTopicStore();

  return useQuery({
    queryKey: ['topic', topicId],
    queryFn: async () => {
      // まずストアから取得を試みる
      const cachedTopic = topics.get(topicId);
      if (cachedTopic) {
        return cachedTopic;
      }

      // なければAPIから取得
      const apiTopics = await TauriApi.getTopics();
      const apiTopic = apiTopics.find((t) => t.id === topicId);

      if (!apiTopic) {
        throw new Error('Topic not found');
      }

      // 統計情報を取得
      const stats = await TauriApi.getTopicStats(apiTopic.id).catch(() => ({
        topic_id: apiTopic.id,
        member_count: apiTopic.member_count ?? 0,
        post_count: apiTopic.post_count ?? 0,
        active_users_24h: 0,
        trending_score: 0,
      }));

      return {
        id: apiTopic.id,
        name: apiTopic.name,
        description: apiTopic.description ?? '',
        tags: [],
        memberCount: stats.member_count,
        postCount: stats.post_count,
        lastActive: apiTopic.updated_at,
        isActive: true,
        createdAt: new Date(apiTopic.created_at * 1000),
        visibility: apiTopic.visibility ?? 'public',
        isJoined: Boolean(apiTopic.is_joined),
      } as Topic;
    },
    enabled: !!topicId,
  });
};

// トピック作成用のミューテーション
export const useCreateTopic = () => {
  const queryClient = useQueryClient();
  const { createTopic } = useTopicStore();

  return useMutation({
    mutationFn: async ({ name, description }: { name: string; description: string }) => {
      return await createTopic(name, description);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['topics'] });
    },
  });
};

// トピック更新用のミューテーション
export const useUpdateTopic = () => {
  const queryClient = useQueryClient();
  const { updateTopicRemote } = useTopicStore();

  return useMutation({
    mutationFn: async ({
      id,
      name,
      description,
    }: {
      id: string;
      name: string;
      description: string;
    }) => {
      return await updateTopicRemote(id, name, description);
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['topics'] });
      queryClient.invalidateQueries({ queryKey: ['topic', variables.id] });
    },
  });
};

// トピック削除用のミューテーション
export const useDeleteTopic = () => {
  const queryClient = useQueryClient();
  const { deleteTopicRemote } = useTopicStore();

  return useMutation({
    mutationFn: async (id: string) => {
      return await deleteTopicRemote(id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['topics'] });
    },
  });
};

// P2Pトピック参加はTopicCard内で直接storeのメソッドを使用
