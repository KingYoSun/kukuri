import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTopicStore } from '@/stores';
import type { Topic } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';

// トピック一覧を取得するフック
export const useTopics = () => {
  const { setTopics } = useTopicStore();

  return useQuery({
    queryKey: ['topics'],
    queryFn: async () => {
      const apiTopics = await TauriApi.getTopics();
      // APIレスポンスをフロントエンドの型に変換
      const topics: Topic[] = apiTopics.map((topic) => ({
        id: topic.id,
        name: topic.name,
        description: topic.description,
        tags: [], // APIにタグ情報がない場合は空配列
        memberCount: 0, // TODO: 実際のメンバー数を取得
        postCount: 0, // TODO: 実際の投稿数を取得
        lastActive: topic.updated_at,
        isActive: true,
        createdAt: new Date(topic.created_at * 1000),
      }));
      setTopics(topics);
      return topics;
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

      return {
        id: apiTopic.id,
        name: apiTopic.name,
        description: apiTopic.description,
        tags: [],
        memberCount: 0,
        postCount: 0,
        lastActive: apiTopic.updated_at,
        isActive: true,
        createdAt: new Date(apiTopic.created_at * 1000),
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
