import { create } from 'zustand';
import type { PostState, Post, PostScope } from './types';
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

const sortPostsDesc = (posts: Post[]): Post[] =>
  [...posts].sort((a, b) => b.created_at - a.created_at);

const upsertPostIntoList = (posts: Post[] | undefined, post: Post): Post[] => {
  const filtered = (posts ?? []).filter((item) => item.id !== post.id);
  return sortPostsDesc([post, ...filtered]);
};

const addPostToCaches = (post: Post) => {
  queryClient.setQueryData<Post[]>(['timeline'], (prev) => upsertPostIntoList(prev, post));
  queryClient.setQueryData<Post[]>(['posts', post.topicId], (prev) =>
    upsertPostIntoList(prev, post),
  );
  queryClient.invalidateQueries({ queryKey: ['topicTimeline', post.topicId] });
  queryClient.invalidateQueries({ queryKey: ['topicThreads', post.topicId] });
  queryClient.invalidateQueries({ queryKey: ['threadPosts', post.topicId] });
};

const replacePostInCaches = (oldId: string, post: Post) => {
  queryClient.setQueryData<Post[]>(['timeline'], (prev) =>
    sortPostsDesc([
      post,
      ...((prev ?? []).filter((item) => item.id !== oldId && item.id !== post.id) ?? []),
    ]),
  );
  queryClient.setQueryData<Post[]>(['posts', post.topicId], (prev) =>
    sortPostsDesc([
      post,
      ...((prev ?? []).filter((item) => item.id !== oldId && item.id !== post.id) ?? []),
    ]),
  );
  queryClient.invalidateQueries({ queryKey: ['topicTimeline', post.topicId] });
  queryClient.invalidateQueries({ queryKey: ['topicThreads', post.topicId] });
  queryClient.invalidateQueries({ queryKey: ['threadPosts', post.topicId] });
};

const removePostFromCaches = (id: string, topicId: string) => {
  queryClient.setQueryData<Post[]>(['timeline'], (prev) =>
    (prev ?? []).filter((item) => item.id !== id),
  );
  queryClient.setQueryData<Post[]>(['posts', topicId], (prev) =>
    (prev ?? []).filter((item) => item.id !== id),
  );
  queryClient.invalidateQueries({ queryKey: ['topicTimeline', topicId] });
  queryClient.invalidateQueries({ queryKey: ['topicThreads', topicId] });
  queryClient.invalidateQueries({ queryKey: ['threadPosts', topicId] });
};

const updatePostLikesInCaches = (postId: string, topicId: string | undefined, likes: number) => {
  if (!topicId) {
    return;
  }
  const updateLikes = (posts?: Post[]) =>
    posts?.map((item) => (item.id === postId ? { ...item, likes } : item)) ?? posts;
  queryClient.setQueryData<Post[]>(['timeline'], (prev) => updateLikes(prev));
  queryClient.setQueryData<Post[]>(['posts', topicId], (prev) => updateLikes(prev));
  queryClient.invalidateQueries({ queryKey: ['topicTimeline', topicId] });
  queryClient.invalidateQueries({ queryKey: ['topicThreads', topicId] });
  queryClient.invalidateQueries({ queryKey: ['threadPosts', topicId] });
};

const resolveReplyCount = (post: Post): number => {
  if (typeof post.replyCount === 'number') {
    return post.replyCount;
  }
  if (Array.isArray(post.replies)) {
    return post.replies.length;
  }
  if (typeof post.replies === 'number') {
    return post.replies;
  }
  return 0;
};

const normalizePost = (post: Post): Post => {
  const replies = Array.isArray(post.replies) ? post.replies : [];
  return {
    ...post,
    replies,
    replyCount: resolveReplyCount(post),
  };
};

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
      scope?: PostScope;
      threadUuid?: string;
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

const removePostCollections = (
  state: PostStore,
  id: string,
): {
  posts: Map<string, Post>;
  postsByTopic: Map<string, string[]>;
  removedTopicId: string;
} | null => {
  const post = state.posts.get(id);
  if (!post) {
    return null;
  }
  const posts = new Map(state.posts);
  posts.delete(id);

  const postsByTopic = new Map(state.postsByTopic);
  const topicPosts = postsByTopic.get(post.topicId) || [];
  postsByTopic.set(
    post.topicId,
    topicPosts.filter((postId) => postId !== id),
  );

  return {
    posts,
    postsByTopic,
    removedTopicId: post.topicId,
  };
};

export const usePostStore = create<PostStore>()((set, get) => ({
  posts: new Map(),
  postsByTopic: new Map(),

  setPosts: (posts: Post[]) => {
    const enrichedPosts = posts.map((post) => normalizePost(enrichPostAuthorMetadata(post)));
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
        apiPosts.map(async (post) =>
          normalizePost(enrichPostAuthorMetadata(await mapPostResponseToDomain(post))),
        ),
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
        toastTitle: '���e�̎擾�Ɏ��s���܂���',
      });
      throw error;
    }
  },

  addPost: (post: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      newPosts.set(post.id, normalizePost(enrichPostAuthorMetadata(post)));

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
      scope?: PostScope;
      threadUuid?: string;
    },
  ) => {
    const offlineStore = useOfflineStore.getState();
    const isOnline = offlineStore.isOnline;
    const scope = options?.scope ?? 'public';
    const explicitThreadUuid = options?.threadUuid?.trim();
    const parentPost = options?.replyTo ? get().posts.get(options.replyTo) : undefined;
    const parentThreadUuid = parentPost?.threadUuid?.trim();
    if (options?.replyTo && !explicitThreadUuid && !parentThreadUuid) {
      throw new Error('reply_to の親投稿がキャッシュにないため threadUuid を解決できません');
    }
    const resolvedThreadUuid = explicitThreadUuid || parentThreadUuid || uuidv4();
    const resolvedThreadNamespace = `${topicId}/threads/${resolvedThreadUuid}`;
    const resolvedThreadRootEventId = options?.replyTo
      ? parentPost?.threadRootEventId || options.replyTo
      : undefined;
    const resolvedThreadParentEventId = options?.replyTo;

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
      threadNamespace: resolvedThreadNamespace,
      threadUuid: resolvedThreadUuid,
      threadRootEventId: resolvedThreadRootEventId ?? tempId,
      threadParentEventId: resolvedThreadParentEventId ?? null,
      scope,
      epoch: null,
      isEncrypted: scope !== 'public',
      created_at: Math.floor(Date.now() / 1000),
      tags: [],
      likes: 0,
      boosts: 0,
      replies: [],
      replyCount: 0,
      isSynced: false, // 未同期として表示
    };

    // 楽観皁E�E�E�E��E�E�E�新: 即座にUIに反映
    set((state) => {
      const newPosts = new Map(state.posts);
      newPosts.set(tempId, optimisticPost);

      const newPostsByTopic = new Map(state.postsByTopic);
      const topicPosts = newPostsByTopic.get(topicId) || [];
      // 新しい投稿を�E頭に追加�E�E�E�E�E�E�E�最新の投稿が上に表示されるよぁE�E�E�E��E�E�E��E�E�E�E�E�E�E�E
      newPostsByTopic.set(topicId, [tempId, ...topicPosts]);

      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic,
      };
    });
    addPostToCaches(optimisticPost);

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
          threadUuid: resolvedThreadUuid,
          replyTo: options?.replyTo,
          quotedPost: options?.quotedPost,
          scope,
        }),
      });

      return optimisticPost;
    }

    // オンラインの場合、バチE�E�E�E��E�E�E�グラウンドでサーバ�Eに送信
    try {
      const apiPost = await TauriApi.createPost({
        content,
        topic_id: topicId,
        thread_uuid: resolvedThreadUuid,
        reply_to: options?.replyTo,
        quoted_post: options?.quotedPost,
        scope,
      });

      const realPost = normalizePost(
        enrichPostAuthorMetadata(await mapPostResponseToDomain(apiPost)),
      );

      // 一晁EDを実際のIDに置き換ぁE
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
      replacePostInCaches(tempId, realPost);

      return realPost;
    } catch (error) {
      const userPubkey =
        currentUser.pubkey || localStorage.getItem('currentUserPubkey') || 'unknown';
      try {
        await offlineStore.saveOfflineAction({
          userPubkey,
          actionType: OfflineActionType.CREATE_POST,
          entityType: EntityType.POST,
          entityId: tempId,
          data: JSON.stringify({
            content,
            topicId,
            threadUuid: resolvedThreadUuid,
            replyTo: options?.replyTo,
            quotedPost: options?.quotedPost,
            scope,
          }),
        });
        errorHandler.log('Failed to create post online, queued for offline sync', error, {
          context: 'PostStore.createPost.offlineFallback',
          showToast: false,
          metadata: { topicId, userPubkey },
        });
        return optimisticPost;
      } catch (offlineError) {
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
        removePostFromCaches(tempId, topicId);

        errorHandler.log('Failed to create post', error, {
          context: 'PostStore.createPost',
          showToast: true,
          toastTitle: '���e�̍쐬�Ɏ��s���܂���',
          metadata: {
            offlineError:
              offlineError instanceof Error ? offlineError.message : String(offlineError),
          },
        });
        throw error;
      }
    }
  },

  updatePost: (id: string, update: Partial<Post>) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      const existing = newPosts.get(id);
      if (existing) {
        newPosts.set(id, normalizePost({ ...existing, ...update }));
      }
      return { posts: newPosts };
    }),

  removePost: (id: string) =>
    set((state) => {
      const next = removePostCollections(state, id);
      if (!next) {
        return state;
      }
      return {
        posts: next.posts,
        postsByTopic: next.postsByTopic,
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
          const next = removePostCollections(state, id);
          if (!next) {
            return state;
          }
          removedTopicId = next.removedTopicId;
          return {
            posts: next.posts,
            postsByTopic: next.postsByTopic,
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
        const next = removePostCollections(state, id);
        if (!next) {
          return state;
        }
        removedTopicId = next.removedTopicId;
        return {
          posts: next.posts,
          postsByTopic: next.postsByTopic,
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
        toastTitle: '���e�̍폜�Ɏ��s���܂���',
      });
      throw error;
    }
  },

  likePost: async (postId: string) => {
    const offlineStore = useOfflineStore.getState();
    const isOnline = offlineStore.isOnline;

    const targetPost = get().posts.get(postId);
    const topicId = targetPost?.topicId;
    const previousLikes = targetPost?.likes || 0;

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
    updatePostLikesInCaches(postId, topicId, previousLikes + 1);

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

    try {
      await TauriApi.likePost(postId);
    } catch (error) {
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
      updatePostLikesInCaches(postId, topicId, previousLikes);

      errorHandler.log('Failed to like post', error, {
        context: 'PostStore.likePost',
        showToast: true,
        toastTitle: '�����˂Ɏ��s���܂���',
      });
      throw error;
    }
  },
  addReply: (parentId: string, reply: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts);
      const parent = newPosts.get(parentId);
      if (parent) {
        const replies = Array.isArray(parent.replies) ? parent.replies : [];
        const updatedParent = normalizePost({
          ...parent,
          replies: [...replies, reply],
          replyCount: resolveReplyCount(parent) + 1,
        });
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

        const repliesSource = Array.isArray(post.replies) ? post.replies : [];
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
          return normalizePost({
            ...post,
            author: updatedAuthor,
            replies: updatedReplies,
            replyCount: post.replyCount ?? updatedReplies.length,
          });
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
