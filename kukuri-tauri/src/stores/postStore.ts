import { create } from 'zustand'
import type { PostState, Post } from './types'
import { TauriApi } from '@/lib/api/tauri'

interface PostStore extends PostState {
  setPosts: (posts: Post[]) => void
  fetchPosts: (topicId?: string, limit?: number, offset?: number) => Promise<void>
  addPost: (post: Post) => void
  createPost: (content: string, topicId: string) => Promise<Post>
  updatePost: (id: string, update: Partial<Post>) => void
  removePost: (id: string) => void
  deletePostRemote: (id: string) => Promise<void>
  likePost: (postId: string) => Promise<void>
  addReply: (parentId: string, reply: Post) => void
  getPostsByTopic: (topicId: string) => Post[]
}

export const usePostStore = create<PostStore>()((set, get) => ({
  posts: new Map(),
  postsByTopic: new Map(),

  setPosts: (posts: Post[]) => {
    const postsMap = new Map(posts.map(p => [p.id, p]))
    const postsByTopicMap = new Map<string, string[]>()
    
    posts.forEach(post => {
      const existing = postsByTopicMap.get(post.topicId) || []
      postsByTopicMap.set(post.topicId, [...existing, post.id])
    })

    set({
      posts: postsMap,
      postsByTopic: postsByTopicMap
    })
  },

  fetchPosts: async (topicId?: string, limit?: number, offset?: number) => {
    try {
      const apiPosts = await TauriApi.getPosts({ topic_id: topicId, limit, offset });
      const posts: Post[] = apiPosts.map(p => ({
        id: p.id,
        content: p.content,
        author: {
          id: p.author_pubkey,
          pubkey: p.author_pubkey,
          npub: p.author_pubkey, // TODO: Convert to npub
          name: '匿名ユーザー',
          displayName: '匿名ユーザー',
          about: '',
          picture: '',
          nip05: ''
        },
        topicId: p.topic_id,
        created_at: p.created_at,
        tags: [],
        likes: p.likes,
        replies: []
      }));
      
      const postsMap = new Map(posts.map(p => [p.id, p]));
      const postsByTopicMap = new Map<string, string[]>();
      
      posts.forEach(post => {
        const existing = postsByTopicMap.get(post.topicId) || [];
        postsByTopicMap.set(post.topicId, [...existing, post.id]);
      });

      set({
        posts: postsMap,
        postsByTopic: postsByTopicMap
      });
    } catch (error) {
      console.error('Failed to fetch posts:', error);
      throw error;
    }
  },

  addPost: (post: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts)
      newPosts.set(post.id, post)
      
      const newPostsByTopic = new Map(state.postsByTopic)
      const topicPosts = newPostsByTopic.get(post.topicId) || []
      newPostsByTopic.set(post.topicId, [...topicPosts, post.id])
      
      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic
      }
    }),

  createPost: async (content: string, topicId: string) => {
    try {
      const apiPost = await TauriApi.createPost({ content, topic_id: topicId });
      const post: Post = {
        id: apiPost.id,
        content: apiPost.content,
        author: {
          id: apiPost.author_pubkey,
          pubkey: apiPost.author_pubkey,
          npub: apiPost.author_pubkey, // TODO: Convert to npub
          name: 'あなた',
          displayName: 'あなた',
          about: '',
          picture: '',
          nip05: ''
        },
        topicId: apiPost.topic_id,
        created_at: apiPost.created_at,
        tags: [],
        likes: apiPost.likes,
        replies: []
      };
      
      set((state) => {
        const newPosts = new Map(state.posts);
        newPosts.set(post.id, post);
        
        const newPostsByTopic = new Map(state.postsByTopic);
        const topicPosts = newPostsByTopic.get(post.topicId) || [];
        newPostsByTopic.set(post.topicId, [...topicPosts, post.id]);
        
        return {
          posts: newPosts,
          postsByTopic: newPostsByTopic
        };
      });
      
      return post;
    } catch (error) {
      console.error('Failed to create post:', error);
      throw error;
    }
  },

  updatePost: (id: string, update: Partial<Post>) =>
    set((state) => {
      const newPosts = new Map(state.posts)
      const existing = newPosts.get(id)
      if (existing) {
        newPosts.set(id, { ...existing, ...update })
      }
      return { posts: newPosts }
    }),

  removePost: (id: string) =>
    set((state) => {
      const post = state.posts.get(id)
      if (!post) return state
      
      const newPosts = new Map(state.posts)
      newPosts.delete(id)
      
      const newPostsByTopic = new Map(state.postsByTopic)
      const topicPosts = newPostsByTopic.get(post.topicId) || []
      newPostsByTopic.set(
        post.topicId, 
        topicPosts.filter(postId => postId !== id)
      )
      
      return {
        posts: newPosts,
        postsByTopic: newPostsByTopic
      }
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
          topicPosts.filter(postId => postId !== id)
        );
        
        return {
          posts: newPosts,
          postsByTopic: newPostsByTopic
        };
      });
    } catch (error) {
      console.error('Failed to delete post:', error);
      throw error;
    }
  },

  likePost: async (postId: string) => {
    try {
      await TauriApi.likePost(postId);
      set((state) => {
        const newPosts = new Map(state.posts);
        const post = newPosts.get(postId);
        if (post) {
          newPosts.set(postId, {
            ...post,
            likes: post.likes + 1
          });
        }
        return { posts: newPosts };
      });
    } catch (error) {
      console.error('Failed to like post:', error);
      throw error;
    }
  },

  addReply: (parentId: string, reply: Post) =>
    set((state) => {
      const newPosts = new Map(state.posts)
      const parent = newPosts.get(parentId)
      if (parent) {
        const updatedParent = {
          ...parent,
          replies: [...(parent.replies || []), reply]
        }
        newPosts.set(parentId, updatedParent)
      }
      return { posts: newPosts }
    }),

  getPostsByTopic: (topicId: string) => {
    const state = get()
    const postIds = state.postsByTopic.get(topicId) || []
    return postIds
      .map(id => state.posts.get(id))
      .filter((post): post is Post => post !== undefined)
      .sort((a, b) => b.created_at - a.created_at)
  }
}))