import { create } from 'zustand'
import type { PostState, Post } from './types'

interface PostStore extends PostState {
  setPosts: (posts: Post[]) => void
  addPost: (post: Post) => void
  updatePost: (id: string, update: Partial<Post>) => void
  removePost: (id: string) => void
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