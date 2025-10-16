import { create } from 'zustand';
import type { PostState, Post } from './types';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { useOfflineStore } from './offlineStore';
import { OfflineActionType, EntityType } from '@/types/offline';
import { v4 as uuidv4 } from 'uuid';
import { pubkeyToNpub } from '@/lib/utils/nostr';

interface PostStore extends PostState {
  setPosts: (posts: Post[]) => void;
  fetchPosts: (topicId?: string, limit?: number, offset?: number) => Promise<void>;
  addPost: (post: Post) => void;
  createPost: (
    content: string,
    topicId: string,
    options?: {
      replyTo?: string;
      quotedPost?: string;
    },
  ) => Promise<Post>;
  updatePost: (id: string, update: Partial<Post>) => void;
  removePost: (id: string) => void;
  deletePostRemote: (id: string) => Promise<void>;
  likePost: (postId: string) => Promise<void>;
  addReply: (parentId: string, reply: Post) => void;
  getPostsByTopic: (topicId: string) => Post[];
  incrementLikes: (postId: string) => void;
  updatePostLikes: (postId: string, likes: number) => void;
}

export const usePostStore = create<PostStore>()((set, get) => ({
  posts: new Map(),
  postsByTopic: new Map(),

  setPosts: (posts: Post[]) => {
    const postsMap = new Map(posts.map((p) => [p.id, p]));
    const postsByTopicMap = new Map<string, string[]>();

    posts.forEach((post) => {
      const existing = postsByTopicMap.get(post.topicId) || [];
      postsByTopicMap.set(post.topicId, [...existing, post.id]);
    });

    set({
      posts: postsMap,
      postsByTopic: postsByTopicMap,
    });
  },

  fetchPosts: async (topicId?: string, limit?: number, offset?: number) => {
    try {
      const apiPosts = await TauriApi.getPosts({ topic_id: topicId, limit, offset });
      const posts: Post[] = await Promise.all(
        apiPosts.map(async (p) => ({
          id: p.id,
          content: p.content,
          author: {
            id: p.author_pubkey,
            pubkey: p.author_pubkey,
            npub: await pubkeyToNpub(p.author_pubkey),
            name: '匿名ユーザー',
            displayName: '匿名ユーザー',
            about: '',
            picture: '',
            nip05: '',
          },
          topicId: p.topic_id,
          created_at: p.created_at,
          tags: [],
          likes: p.likes,
          boosts: p.boosts || 0,
          replies: [],
          isSynced: p.is_synced ?? true, // DBのis_syncedフィールドを使用（未定義の場合はtrue）
        })),
      );

      const postsMap = new Map(posts.map((p) => [p.id, p]));
      const postsByTopicMap = new Map<string, string[]>();

      posts.forEach((post) => {
        const existing = postsByTopicMap.get(post.topicId) || [];
        postsByTopicMap.set(post.topicId, [...existing, post.id]);
      });

      set({
        posts: postsMap,
        postsByTopic: postsByTopicMap,
      });
    } catch (error) {
      errorHandler.log('Failed to fetch posts', error, {
        context: 'PostStore.fetchPosts',
        showToast: true,
        toastTitle: '投稿の取得に失敗しました',
      });
      throw error;
    }
  },

  addPost: (post: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      newPosts.set(post.id, post);

      const newPostsByTopic = new Map(state.postsByTopic);
      const topicPosts = newPostsByTopic.get(post.topicId) || [];
      newPostsByTopic.set(post.topicId, [...topicPosts, post.id]);

      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic,
      };
    }),

  createPost: async (
    content: string,
    topicId: string,
    options?: {
      replyTo?: string;
      quotedPost?: string;
    },
  ) => {
    const offlineStore = useOfflineStore.getState();
    const isOnline = offlineStore.isOnline;

    // 楽観的投稿データを作成
    const tempId = `temp-${uuidv4()}`;
    const optimisticPost: Post = {
      id: tempId,
      content,
      author: {
        id: 'current-user', // 実際のユーザー情報を取得する必要があります
        pubkey: 'current-user',
        npub: 'current-user',
        name: 'あなた',
        displayName: 'あなた',
        about: '',
        picture: '',
        nip05: '',
      },
      topicId,
      created_at: Date.now(),
      tags: [],
      likes: 0,
      boosts: 0,
      replies: [],
      isSynced: false, // 未同期として表示
    };

    // 楽観的更新: 即座にUIに反映
    set((state) => {
      const newPosts = new Map(state.posts);
      newPosts.set(tempId, optimisticPost);

      const newPostsByTopic = new Map(state.postsByTopic);
      const topicPosts = newPostsByTopic.get(topicId) || [];
      // 新しい投稿を先頭に追加（最新の投稿が上に表示されるように）
      newPostsByTopic.set(topicId, [tempId, ...topicPosts]);

      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic,
      };
    });

    // オフラインの場合、アクションを保存して後で同期
    if (!isOnline) {
      const userPubkey = localStorage.getItem('currentUserPubkey') || 'unknown';
      await offlineStore.saveOfflineAction({
        userPubkey,
        actionType: OfflineActionType.CREATE_POST,
        entityType: EntityType.POST,
        entityId: tempId,
        data: JSON.stringify({
          content,
          topicId,
          replyTo: options?.replyTo,
          quotedPost: options?.quotedPost,
        }),
      });

      return optimisticPost;
    }

    // オンラインの場合、バックグラウンドでサーバーに送信
    try {
      const apiPost = await TauriApi.createPost({
        content,
        topic_id: topicId,
        reply_to: options?.replyTo,
        quoted_post: options?.quotedPost,
      });

      const realPost: Post = {
        id: apiPost.id,
        content: apiPost.content,
        author: {
          id: apiPost.author_pubkey,
          pubkey: apiPost.author_pubkey,
          npub: apiPost.author_pubkey,
          name: 'あなた',
          displayName: 'あなた',
          about: '',
          picture: '',
          nip05: '',
        },
        topicId: apiPost.topic_id,
        created_at: apiPost.created_at,
        tags: [],
        likes: apiPost.likes,
        boosts: apiPost.boosts || 0,
        replies: [],
        isSynced: true, // サーバーに送信成功
      };

      // 一時IDを実際のIDに置き換え
      set((state) => {
        const newPosts = new Map(state.posts);
        newPosts.delete(tempId);
        newPosts.set(realPost.id, realPost);

        const newPostsByTopic = new Map(state.postsByTopic);
        const topicPosts = newPostsByTopic.get(topicId) || [];
        const updatedTopicPosts = topicPosts.map((id) => (id === tempId ? realPost.id : id));
        newPostsByTopic.set(topicId, updatedTopicPosts);

        return {
          posts: newPosts,
          postsByTopic: newPostsByTopic,
        };
      });

      return realPost;
    } catch (error) {
      // 失敗した場合、ロールバック
      set((state) => {
        const newPosts = new Map(state.posts);
        newPosts.delete(tempId);

        const newPostsByTopic = new Map(state.postsByTopic);
        const topicPosts = newPostsByTopic.get(topicId) || [];
        const updatedTopicPosts = topicPosts.filter((id) => id !== tempId);
        newPostsByTopic.set(topicId, updatedTopicPosts);

        return {
          posts: newPosts,
          postsByTopic: newPostsByTopic,
        };
      });

      errorHandler.log('Failed to create post', error, {
        context: 'PostStore.createPost',
        showToast: true,
        toastTitle: '投稿の作成に失敗しました',
      });
      throw error;
    }
  },

  updatePost: (id: string, update: Partial<Post>) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      const existing = newPosts.get(id);
      if (existing) {
        newPosts.set(id, { ...existing, ...update });
      }
      return { posts: newPosts };
    }),

  removePost: (id: string) =>
    set((state) => {
      const post = state.posts.get(id);
      if (!post) return state;

      const newPosts = new Map(state.posts);
      newPosts.delete(id);

      const newPostsByTopic = new Map(state.postsByTopic);
      const topicPosts = newPostsByTopic.get(post.topicId) || [];
      newPostsByTopic.set(
        post.topicId,
        topicPosts.filter((postId) => postId !== id),
      );

      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic,
      };
    }),

  deletePostRemote: async (id: string) => {
    try {
      await TauriApi.deletePost(id);
      set((state) => {
        const post = state.posts.get(id);
        if (!post) return state;

        const newPosts = new Map(state.posts);
        newPosts.delete(id);

        const newPostsByTopic = new Map(state.postsByTopic);
        const topicPosts = newPostsByTopic.get(post.topicId) || [];
        newPostsByTopic.set(
          post.topicId,
          topicPosts.filter((postId) => postId !== id),
        );

        return {
          posts: newPosts,
          postsByTopic: newPostsByTopic,
        };
      });
    } catch (error) {
      errorHandler.log('Failed to delete post', error, {
        context: 'PostStore.deletePostRemote',
        showToast: true,
        toastTitle: '投稿の削除に失敗しました',
      });
      throw error;
    }
  },

  likePost: async (postId: string) => {
    const offlineStore = useOfflineStore.getState();
    const isOnline = offlineStore.isOnline;

    // 楽観的更新: 即座にUIに反映
    const previousLikes = get().posts.get(postId)?.likes || 0;
    set((state) => {
      const newPosts = new Map(state.posts);
      const post = newPosts.get(postId);
      if (post) {
        newPosts.set(postId, {
          ...post,
          likes: post.likes + 1,
        });
      }
      return { posts: newPosts };
    });

    // オフラインの場合、アクションを保存して後で同期
    if (!isOnline) {
      const userPubkey = localStorage.getItem('currentUserPubkey') || 'unknown';
      await offlineStore.saveOfflineAction({
        userPubkey,
        actionType: OfflineActionType.LIKE_POST,
        entityType: EntityType.POST,
        entityId: postId,
        data: JSON.stringify({ postId }),
      });
      return;
    }

    // オンラインの場合、バックグラウンドでサーバーに送信
    try {
      await TauriApi.likePost(postId);
      // 成功した場合は特に何もしない（既に楽観的更新済み）
    } catch (error) {
      // 失敗した場合、ロールバック
      set((state) => {
        const newPosts = new Map(state.posts);
        const post = newPosts.get(postId);
        if (post) {
          newPosts.set(postId, {
            ...post,
            likes: previousLikes,
          });
        }
        return { posts: newPosts };
      });

      errorHandler.log('Failed to like post', error, {
        context: 'PostStore.likePost',
        showToast: true,
        toastTitle: 'いいねに失敗しました',
      });
      throw error;
    }
  },

  addReply: (parentId: string, reply: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      const parent = newPosts.get(parentId);
      if (parent) {
        const updatedParent = {
          ...parent,
          replies: [...(parent.replies || []), reply],
        };
        newPosts.set(parentId, updatedParent);
      }
      return { posts: newPosts };
    }),

  getPostsByTopic: (topicId: string) => {
    const state = get();
    const postIds = state.postsByTopic.get(topicId) || [];
    return postIds
      .map((id) => state.posts.get(id))
      .filter((post): post is Post => post !== undefined)
      .sort((a, b) => b.created_at - a.created_at);
  },

  incrementLikes: (postId: string) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      const post = newPosts.get(postId);
      if (post) {
        newPosts.set(postId, {
          ...post,
          likes: post.likes + 1,
        });
      }
      return { posts: newPosts };
    }),

  updatePostLikes: (postId: string, likes: number) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      const post = newPosts.get(postId);
      if (post) {
        newPosts.set(postId, {
          ...post,
          likes,
        });
      }
      return { posts: newPosts };
    }),
}));
