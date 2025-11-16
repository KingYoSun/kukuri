import { create } from 'zustand';
import type { PostState, Post } from './types';
import { TauriApi, type GetPostsRequest } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';
import { useOfflineStore } from './offlineStore';
import { OfflineActionType, EntityType } from '@/types/offline';
import { v4 as uuidv4 } from 'uuid';
import { mapPostResponseToDomain, enrichPostAuthorMetadata } from '@/lib/posts/postMapper';
import { applyKnownUserMetadata } from '@/lib/profile/userMetadata';
import { useAuthStore } from './authStore';
import { useTopicStore } from './topicStore';
import { invalidatePostCaches } from '@/lib/posts/cacheUtils';
import { queryClient } from '@/lib/queryClient';

type DeletePostRemoteInput = {
  id: string;
  topicId?: string | null;
  authorPubkey?: string | null;
};

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
  deletePostRemote: (input: DeletePostRemoteInput) => Promise<void>;
  likePost: (postId: string) => Promise<void>;
  addReply: (parentId: string, reply: Post) => void;
  getPostsByTopic: (topicId: string) => Post[];
  incrementLikes: (postId: string) => void;
  updatePostLikes: (postId: string, likes: number) => void;
  refreshAuthorMetadata: (npub: string) => void;
}

export const usePostStore = create<PostStore>()((set, get) => ({
  posts: new Map(),
  postsByTopic: new Map(),

  setPosts: (posts: Post[]) => {
    const enrichedPosts = posts.map((post) => enrichPostAuthorMetadata(post));
    const postsMap = new Map(enrichedPosts.map((p) => [p.id, p]));
    const postsByTopicMap = new Map<string, string[]>();

    enrichedPosts.forEach((post) => {
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
      const requestPayload: GetPostsRequest = {
        topic_id: topicId,
      };
      if (limit !== undefined || offset !== undefined) {
        requestPayload.pagination = { limit, offset };
      }
      const apiPosts = await TauriApi.getPosts(requestPayload);
      const posts: Post[] = await Promise.all(
        apiPosts.map(async (post) => enrichPostAuthorMetadata(await mapPostResponseToDomain(post))),
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
        toastTitle: '謚慕ｨｿ縺ｮ蜿門ｾ励↓螟ｱ謨励＠縺ｾ縺励◆',
      });
      throw error;
    }
  },

  addPost: (post: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      newPosts.set(post.id, enrichPostAuthorMetadata(post));

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

    const authState = useAuthStore.getState();
    const currentUser = authState.currentUser;

    if (!currentUser) {
      throw new Error('Not authenticated');
    }

    const author = applyKnownUserMetadata({
      ...currentUser,
    });

    const tempId = `temp-${uuidv4()}`;
    const optimisticPost: Post = {
      id: tempId,
      content,
      author,
      topicId,
      created_at: Math.floor(Date.now() / 1000),
      tags: [],
      likes: 0,
      boosts: 0,
      replies: [],
      isSynced: false, // 譛ｪ蜷梧悄縺ｨ縺励※陦ｨ遉ｺ
    };

    // 讌ｽ隕ｳ逧・峩譁ｰ: 蜊ｳ蠎ｧ縺ｫUI縺ｫ蜿肴丐
    set((state) => {
      const newPosts = new Map(state.posts);
      newPosts.set(tempId, optimisticPost);

      const newPostsByTopic = new Map(state.postsByTopic);
      const topicPosts = newPostsByTopic.get(topicId) || [];
      // 譁ｰ縺励＞謚慕ｨｿ繧貞・鬆ｭ縺ｫ霑ｽ蜉・域怙譁ｰ縺ｮ謚慕ｨｿ縺御ｸ翫↓陦ｨ遉ｺ縺輔ｌ繧九ｈ縺・↓・・
      newPostsByTopic.set(topicId, [tempId, ...topicPosts]);

      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic,
      };
    });

    // 繧ｪ繝輔Λ繧､繝ｳ縺ｮ蝣ｴ蜷医√い繧ｯ繧ｷ繝ｧ繝ｳ繧剃ｿ晏ｭ倥＠縺ｦ蠕後〒蜷梧悄
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

    // 繧ｪ繝ｳ繝ｩ繧､繝ｳ縺ｮ蝣ｴ蜷医√ヰ繝・け繧ｰ繝ｩ繧ｦ繝ｳ繝峨〒繧ｵ繝ｼ繝舌・縺ｫ騾∽ｿ｡
    try {
      const apiPost = await TauriApi.createPost({
        content,
        topic_id: topicId,
        reply_to: options?.replyTo,
        quoted_post: options?.quotedPost,
      });

      const realPost = enrichPostAuthorMetadata(await mapPostResponseToDomain(apiPost));

      // 荳譎・D繧貞ｮ滄圀縺ｮID縺ｫ鄂ｮ縺肴鋤縺・
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
      // 螟ｱ謨励＠縺溷ｴ蜷医√Ο繝ｼ繝ｫ繝舌ャ繧ｯ
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
        toastTitle: '謚慕ｨｿ縺ｮ菴懈・縺ｫ螟ｱ謨励＠縺ｾ縺励◆',
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

  deletePostRemote: async ({
    id,
    topicId: fallbackTopicId,
    authorPubkey: fallbackAuthorPubkey,
  }: DeletePostRemoteInput) => {
    try {
      const offlineStore = useOfflineStore.getState();
      const isOnline = offlineStore.isOnline;

      const authState = useAuthStore.getState();
      const currentUser = authState.currentUser;

      if (!currentUser) {
        throw new Error('Not authenticated');
      }
      const existingPost = get().posts.get(id);
      const resolvedTopicId = existingPost?.topicId ?? fallbackTopicId ?? null;
      const resolvedAuthorPubkey = existingPost?.author.pubkey ?? fallbackAuthorPubkey ?? null;

      const invalidateCaches = () => {
        invalidatePostCaches(queryClient, {
          id,
          topicId: resolvedTopicId ?? undefined,
          authorPubkey: resolvedAuthorPubkey ?? undefined,
        });
      };

      if (!isOnline) {
        const payload: Record<string, unknown> = { postId: id };
        if (resolvedTopicId) {
          payload.topicId = resolvedTopicId;
        }
        if (resolvedAuthorPubkey) {
          payload.authorPubkey = resolvedAuthorPubkey;
        }
        await offlineStore.saveOfflineAction({
          userPubkey: currentUser.pubkey,
          actionType: OfflineActionType.DELETE_POST,
          entityType: EntityType.POST,
          entityId: id,
          data: JSON.stringify(payload),
        });
        let removedTopicId: string | null = null;
        set((state) => {
          const post = state.posts.get(id);
          if (!post) return state;
          removedTopicId = post.topicId;

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
        if (removedTopicId) {
          const topicStore = useTopicStore.getState();
          topicStore.updateTopicPostCount?.(removedTopicId, -1);
        }
        invalidateCaches();
        return;
      }

      await TauriApi.deletePost(id);
      let removedTopicId: string | null = null;
      set((state) => {
        const post = state.posts.get(id);
        if (!post) return state;
        removedTopicId = post.topicId;

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
      if (removedTopicId) {
        const topicStore = useTopicStore.getState();
        topicStore.updateTopicPostCount?.(removedTopicId, -1);
      }
      invalidateCaches();
    } catch (error) {
      errorHandler.log('Failed to delete post', error, {
        context: 'PostStore.deletePostRemote',
        showToast: true,
        toastTitle: '謚慕ｨｿ縺ｮ蜑企勁縺ｫ螟ｱ謨励＠縺ｾ縺励◆',
      });
      throw error;
    }
  },

  likePost: async (postId: string) => {
    const offlineStore = useOfflineStore.getState();
    const isOnline = offlineStore.isOnline;

    // 讌ｽ隕ｳ逧・峩譁ｰ: 蜊ｳ蠎ｧ縺ｫUI縺ｫ蜿肴丐
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

    // 繧ｪ繝輔Λ繧､繝ｳ縺ｮ蝣ｴ蜷医√い繧ｯ繧ｷ繝ｧ繝ｳ繧剃ｿ晏ｭ倥＠縺ｦ蠕後〒蜷梧悄
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

    // 繧ｪ繝ｳ繝ｩ繧､繝ｳ縺ｮ蝣ｴ蜷医√ヰ繝・け繧ｰ繝ｩ繧ｦ繝ｳ繝峨〒繧ｵ繝ｼ繝舌・縺ｫ騾∽ｿ｡
    try {
      await TauriApi.likePost(postId);
      // 謌仙粥縺励◆蝣ｴ蜷医・迚ｹ縺ｫ菴輔ｂ縺励↑縺・ｼ域里縺ｫ讌ｽ隕ｳ逧・峩譁ｰ貂医∩・・
    } catch (error) {
      // 螟ｱ謨励＠縺溷ｴ蜷医√Ο繝ｼ繝ｫ繝舌ャ繧ｯ
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
        toastTitle: '縺・＞縺ｭ縺ｫ螟ｱ謨励＠縺ｾ縺励◆',
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

  refreshAuthorMetadata: (npub: string) =>
    set((state) => {
      if (!npub) {
        return state;
      }

      const normalized = npub.toLowerCase();
      let hasChanges = false;

      const updatePostAuthor = (post: Post): Post => {
        const authorNpub = post.author.npub?.toLowerCase() ?? '';
        const authorPubkey = post.author.pubkey?.toLowerCase() ?? '';
        const shouldUpdate = authorNpub === normalized || authorPubkey === normalized;

        const repliesSource = post.replies ?? [];
        const updatedReplies = repliesSource.map(updatePostAuthor);
        const repliesChanged =
          updatedReplies.length !== repliesSource.length ||
          updatedReplies.some((reply, index) => reply !== repliesSource[index]);

        let updatedAuthor = post.author;
        let authorChanged = false;

        if (shouldUpdate) {
          const applied = applyKnownUserMetadata(post.author);
          authorChanged = JSON.stringify(applied) !== JSON.stringify(post.author);
          updatedAuthor = applied;
        }

        if (authorChanged || repliesChanged) {
          hasChanges = true;
          return {
            ...post,
            author: updatedAuthor,
            replies: updatedReplies,
          };
        }

        return post;
      };

      const newPosts = new Map(state.posts);
      newPosts.forEach((post, id) => {
        const updated = updatePostAuthor(post);
        if (updated !== post) {
          newPosts.set(id, updated);
        }
      });

      if (!hasChanges) {
        return state;
      }

      return {
        posts: newPosts,
      };
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

useAuthStore.subscribe((state, previousState) => {
  const nextNpub = state.currentUser?.npub;
  const previousNpub = previousState?.currentUser?.npub;
  if (nextNpub && nextNpub !== previousNpub) {
    usePostStore.getState().refreshAuthorMetadata(nextNpub);
  }
});

useAuthStore.subscribe((state, previousState) => {
  const prevAccounts = previousState?.accounts ?? [];
  const hasAccountsChanged =
    prevAccounts.length !== state.accounts.length ||
    prevAccounts.some((account, index) => account.npub !== state.accounts[index]?.npub);
  if (!hasAccountsChanged) {
    return;
  }
  const refresh = usePostStore.getState().refreshAuthorMetadata;
  state.accounts.forEach((account) => {
    refresh(account.npub);
  });
});
