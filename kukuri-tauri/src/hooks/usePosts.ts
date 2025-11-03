import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { usePostStore } from '@/stores';
import type { Post } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import { useAuthStore } from '@/stores';

// すべての投稿を取得
export const usePosts = () => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', 'all'],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({ limit: 1000 });
      // APIレスポンスをフロントエンドの型に変換
      const posts: Post[] = await Promise.all(
        apiPosts.map((post) => mapPostResponseToDomain(post)),
      );
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
      const apiPosts = await TauriApi.getPosts({ limit: 50 });
      // APIレスポンスをフロントエンドの型に変換
      const posts: Post[] = await Promise.all(
        apiPosts.map((post) => mapPostResponseToDomain(post)),
      );
      setPosts(posts);
      return posts;
    },
    refetchInterval: 30000, // 30秒ごとに更新
  });
};

const createPost = async (postData: { content: string; topicId: string }): Promise<Post> => {
  const currentUser = useAuthStore.getState().currentUser;
  if (!currentUser) throw new Error('Not authenticated');

  const apiPost = await TauriApi.createPost({
    content: postData.content,
    topic_id: postData.topicId,
  });

  return await mapPostResponseToDomain(apiPost);
};

export const usePostsByTopic = (topicId: string) => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', topicId],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({ topic_id: topicId, limit: 50 });
      const posts: Post[] = await Promise.all(
        apiPosts.map((post) => mapPostResponseToDomain(post)),
      );
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
