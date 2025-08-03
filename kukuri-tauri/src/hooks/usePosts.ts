import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { usePostStore } from '@/stores';
import type { Post } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { useAuthStore } from '@/stores';

// すべての投稿を取得
export const usePosts = () => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', 'all'],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({ limit: 1000 });
      // APIレスポンスをフロントエンドの型に変換
      const posts: Post[] = apiPosts.map((post) => ({
        id: post.id,
        content: post.content,
        author: {
          id: post.author_pubkey,
          pubkey: post.author_pubkey,
          npub: `npub${post.author_pubkey.slice(0, 8)}...`, // 簡易的なnpub表示
          name: 'ユーザー',
          displayName: 'ユーザー',
          picture: '',
          about: '',
          nip05: '',
        },
        topicId: post.topic_id || '',
        created_at: post.created_at,
        tags: [],
        likes: post.likes,
        boosts: post.boosts || 0,
        replies: [],
        isSynced: post.is_synced,
      }));
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
      const posts: Post[] = apiPosts.map((post) => ({
        id: post.id,
        content: post.content,
        author: {
          id: post.author_pubkey,
          pubkey: post.author_pubkey,
          npub: `npub${post.author_pubkey.slice(0, 8)}...`, // 簡易的なnpub表示
          name: 'ユーザー',
          displayName: 'ユーザー',
          picture: '',
          about: '',
          nip05: '',
        },
        topicId: post.topic_id || '',
        created_at: post.created_at,
        tags: [],
        likes: post.likes,
        boosts: post.boosts || 0,
        replies: [],
        isSynced: post.is_synced,
      }));
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

  return {
    id: apiPost.id,
    content: apiPost.content,
    author: currentUser,
    topicId: apiPost.topic_id,
    created_at: apiPost.created_at,
    tags: [],
    likes: apiPost.likes,
    boosts: apiPost.boosts || 0,
    replies: [],
    isSynced: apiPost.is_synced,
  };
};

export const usePostsByTopic = (topicId: string) => {
  const { setPosts } = usePostStore();

  return useQuery({
    queryKey: ['posts', topicId],
    queryFn: async () => {
      const apiPosts = await TauriApi.getPosts({ topic_id: topicId, limit: 50 });
      const posts: Post[] = apiPosts.map((post) => ({
        id: post.id,
        content: post.content,
        author: {
          id: post.author_pubkey,
          pubkey: post.author_pubkey,
          npub: `npub${post.author_pubkey.slice(0, 8)}...`,
          name: 'ユーザー',
          displayName: 'ユーザー',
          picture: '',
          about: '',
          nip05: '',
        },
        topicId: post.topic_id,
        created_at: post.created_at,
        tags: [],
        likes: post.likes,
        boosts: 0,
        replies: [],
      }));
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
