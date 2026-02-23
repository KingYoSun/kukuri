import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';
import { usePostStore } from '@/stores';
import type { Post } from '@/stores';
import {
  TauriApi,
  type Post as ApiPost,
  type TopicTimelineEntry as ApiTopicTimelineEntry,
} from '@/lib/api/tauri';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import { useAuthStore } from '@/stores';
import { useTopicStore } from '@/stores/topicStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { errorHandler } from '@/lib/errorHandler';
import { invalidatePostCaches } from '@/lib/posts/cacheUtils';
import { toast } from 'sonner';
import i18n from '@/i18n';
import { v4 as uuidv4 } from 'uuid';
import type { TimelineUpdateMode } from '@/stores/uiStore';

export interface TopicTimelineEntry {
  threadUuid: string;
  parentPost: Post;
  firstReply: Post | null;
  replyCount: number;
  lastActivityAt: number;
}

const mapTopicTimelineEntry = async (entry: ApiTopicTimelineEntry): Promise<TopicTimelineEntry> => {
  const [parentPost, firstReply] = await Promise.all([
    mapPostResponseToDomain(entry.parent_post),
    entry.first_reply ? mapPostResponseToDomain(entry.first_reply) : Promise.resolve(null),
  ]);

  return {
    threadUuid: entry.thread_uuid,
    parentPost,
    firstReply,
    replyCount: entry.reply_count,
    lastActivityAt: entry.last_activity_at,
  };
};

export const collectTimelineStorePosts = (entries: TopicTimelineEntry[]): Post[] => {
  const postMap = new Map<string, Post>();
  entries.forEach((entry) => {
    postMap.set(entry.parentPost.id, entry.parentPost);
    if (entry.firstReply) {
      postMap.set(entry.firstReply.id, entry.firstReply);
    }
  });
  return [...postMap.values()].sort((a, b) => b.created_at - a.created_at);
};

const mapApiPosts = async (apiPosts: ApiPost[]): Promise<Post[]> =>
  await Promise.all(apiPosts.map((post) => mapPostResponseToDomain(post)));

const fetchTopicTimelineEntries = async (topicId: string): Promise<TopicTimelineEntry[]> => {
  const apiEntries = await TauriApi.getTopicTimeline({
    topic_id: topicId,
    pagination: { limit: 50 },
  });
  return await Promise.all(apiEntries.map((entry) => mapTopicTimelineEntry(entry)));
};

// すべての投稿を取得
export const usePosts = () => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', 'all'],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({
        pagination: { limit: 1000 },
      });
      const posts = await mapApiPosts(apiPosts);
      setPosts(posts);
      return posts;
    },
    staleTime: 30000, // 30秒
  });
};

// タイムライン用の投稿取得
export const useTimelinePosts = () => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['timeline'],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({
        pagination: { limit: 50 },
      });
      const posts = await mapApiPosts(apiPosts);
      setPosts(posts);
      return posts;
    },
    refetchInterval: 30000, // 30秒ごとに更新
  });
};

export const useTopicTimeline = (topicId: string, mode: TimelineUpdateMode = 'standard') => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['topicTimeline', topicId],
    queryFn: async () => {
      const entries = await fetchTopicTimelineEntries(topicId);
      setPosts(collectTimelineStorePosts(entries));
      return entries;
    },
    enabled: !!topicId,
    refetchInterval: mode === 'standard' ? 30000 : false,
  });
};

export const useTopicThreads = (topicId: string) => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['topicThreads', topicId],
    queryFn: async () => {
      const entries = await fetchTopicTimelineEntries(topicId);
      setPosts(collectTimelineStorePosts(entries));
      return entries;
    },
    enabled: !!topicId,
    refetchInterval: 30000,
  });
};

export const useThreadPosts = (topicId: string, threadUuid: string) => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['threadPosts', topicId, threadUuid],
    queryFn: async () => {
      const apiPosts = await TauriApi.getThreadPosts({
        topic_id: topicId,
        thread_uuid: threadUuid,
        pagination: { limit: 200 },
      });
      const posts = await mapApiPosts(apiPosts);
      setPosts(posts);
      return posts;
    },
    enabled: !!topicId && !!threadUuid,
    refetchInterval: 30000,
  });
};

const createPost = async (postData: { content: string; topicId: string }): Promise<Post> => {
  const currentUser = useAuthStore.getState().currentUser;
  if (!currentUser) throw new Error('Not authenticated');

  const apiPost = await TauriApi.createPost({
    content: postData.content,
    topic_id: postData.topicId,
    thread_uuid: uuidv4(),
  });

  return await mapPostResponseToDomain(apiPost);
};

export const usePostsByTopic = (topicId: string) => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', topicId],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({
        topic_id: topicId,
        pagination: { limit: 50 },
      });
      const posts = await mapApiPosts(apiPosts);
      setPosts(posts);
      return posts;
    },
    enabled: !!topicId,
  });
};

export const useCreatePost = () => {
  const queryClient = useQueryClient();
  const { addPost } = usePostStore();

  return useMutation({
    mutationFn: createPost,
    onSuccess: (newPost) => {
      addPost(newPost);
      queryClient.invalidateQueries({ queryKey: ['posts', newPost.topicId] });
    },
  });
};

type DeletePostMutationInput =
  | Post
  | {
      id: string;
      topicId?: string | null;
      authorPubkey?: string | null;
    };

const isFullPost = (target: DeletePostMutationInput): target is Post => {
  return (target as Post).author !== undefined;
};

export const useDeletePost = () => {
  const queryClient = useQueryClient();
  const deletePostRemote = usePostStore((state) => state.deletePostRemote);
  const updateTopicPostCount = useTopicStore((state) => state.updateTopicPostCount);
  const isOnline = useOfflineStore((state) => state.isOnline);

  const deletePostMutation = useMutation({
    mutationFn: async (target: DeletePostMutationInput) => {
      const topicId = isFullPost(target) ? target.topicId : (target.topicId ?? null);
      const authorPubkey = isFullPost(target)
        ? target.author.pubkey
        : (target.authorPubkey ?? null);
      await deletePostRemote({
        id: target.id,
        topicId,
        authorPubkey,
      });
      return target;
    },
    onSuccess: (target) => {
      const topicId = isFullPost(target) ? target.topicId : (target.topicId ?? undefined);
      const authorPubkey = isFullPost(target)
        ? target.author.pubkey
        : (target.authorPubkey ?? undefined);

      if (isFullPost(target)) {
        updateTopicPostCount(target.topicId, -1);
      }
      invalidatePostCaches(queryClient, {
        id: target.id,
        topicId,
        authorPubkey,
      });
      if (isOnline) {
        toast.success(i18n.t('posts.deleted'));
      } else {
        toast.success(i18n.t('posts.deleteQueued'));
        errorHandler.info('Post.delete_offline_enqueued', 'useDeletePost');
      }
    },
    onError: (error, post) => {
      errorHandler.log('Post.delete_failed', error, {
        context: 'useDeletePost',
        metadata: post
          ? {
              postId: post.id,
              topicId: isFullPost(post) ? post.topicId : (post.topicId ?? undefined),
            }
          : undefined,
      });
      toast.error(i18n.t('posts.deleteFailed'));
    },
  });

  const manualRetryDelete = useCallback(
    async (input: { postId: string; topicId?: string | null; authorPubkey?: string | null }) => {
      const existingPost = usePostStore.getState().posts.get(input.postId);
      if (existingPost) {
        await deletePostMutation.mutateAsync(existingPost);
        return;
      }
      await deletePostMutation.mutateAsync({
        id: input.postId,
        topicId: input.topicId,
        authorPubkey: input.authorPubkey,
      });
    },
    [deletePostMutation],
  );

  return {
    ...deletePostMutation,
    manualRetryDelete,
  };
};
