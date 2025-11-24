import { useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTopicStore } from '@/stores';
import type { Topic } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';

export const useTopics = () => {
  const { setTopics } = useTopicStore();
  const refreshPendingTopics = useTopicStore((state) => state.refreshPendingTopics);
  const storeTopics = useTopicStore((state) => state.topics);
  const keepLocalTopics =
    typeof window !== 'undefined' &&
    (window as unknown as { __E2E_KEEP_LOCAL_TOPICS__?: boolean }).__E2E_KEEP_LOCAL_TOPICS__ ===
      true;

  useEffect(() => {
    if (typeof refreshPendingTopics !== 'function') {
      return;
    }
    void refreshPendingTopics();
  }, [refreshPendingTopics]);

  const mergeWithExisting = (apiTopics: Topic[]): Topic[] => {
    if (!keepLocalTopics) {
      return apiTopics;
    }
    const existing = useTopicStore.getState().topics;
    const merged = new Map<string, Topic>();
    apiTopics.forEach((topic) => merged.set(topic.id, topic));
    existing.forEach((topic, id) => {
      if (!merged.has(id)) {
        merged.set(id, topic);
      }
    });
    return Array.from(merged.values());
  };

  const queryResult = useQuery({
    queryKey: ['topics'],
    queryFn: async () => {
      const apiTopics = await TauriApi.getTopics();

      const topicsWithStats = await Promise.all(
        apiTopics.map(async (topic) => {
          try {
            const stats = await TauriApi.getTopicStats(topic.id);
            return {
              id: topic.id,
              name: topic.name,
              description: topic.description ?? '',
              tags: [],
              memberCount: stats.member_count,
              postCount: stats.post_count,
              lastActive: topic.updated_at,
              isActive: true,
              createdAt: new Date(topic.created_at * 1000),
              visibility: topic.visibility ?? 'public',
              isJoined: Boolean(topic.is_joined),
            } as Topic;
          } catch (error) {
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

      const merged = mergeWithExisting(topicsWithStats);
      setTopics(merged);
      return merged;
    },
    refetchInterval: keepLocalTopics ? false : 30000,
  });

  const fallbackTopics = Array.from(storeTopics.values());
  return {
    ...queryResult,
    data: keepLocalTopics ? fallbackTopics : queryResult.data,
  };
};

export const useTopic = (topicId: string) => {
  const { topics } = useTopicStore();

  return useQuery({
    queryKey: ['topic', topicId],
    queryFn: async () => {
      const cachedTopic = topics.get(topicId);
      if (cachedTopic) {
        return cachedTopic;
      }

      const apiTopics = await TauriApi.getTopics();
      const apiTopic = apiTopics.find((t) => t.id === topicId);

      if (!apiTopic) {
        throw new Error('Topic not found');
      }

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

// P2P topic join uses TopicCard side store methods directly
