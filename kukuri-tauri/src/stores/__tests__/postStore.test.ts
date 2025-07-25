import { describe, it, expect, beforeEach } from 'vitest'
import { usePostStore } from '../postStore'
import type { Post } from '../types'

describe('postStore', () => {
  const mockPost1: Post = {
    id: 'post1',
    pubkey: 'npub123',
    content: 'テスト投稿1',
    topicId: 'topic1',
    created_at: Date.now(),
    tags: [],
  }

  const mockPost2: Post = {
    id: 'post2',
    pubkey: 'npub456',
    content: 'テスト投稿2',
    topicId: 'topic1',
    created_at: Date.now() - 1000,
    tags: [],
  }

  const mockPost3: Post = {
    id: 'post3',
    pubkey: 'npub789',
    content: 'テスト投稿3',
    topicId: 'topic2',
    created_at: Date.now() - 2000,
    tags: [],
  }

  beforeEach(() => {
    usePostStore.setState({
      posts: new Map(),
      postsByTopic: new Map(),
    })
  })

  it('初期状態が正しく設定されていること', () => {
    const state = usePostStore.getState()
    expect(state.posts.size).toBe(0)
    expect(state.postsByTopic.size).toBe(0)
  })

  it('setPostsメソッドが正しく動作すること', () => {
    usePostStore.getState().setPosts([mockPost1, mockPost2, mockPost3])
    
    const state = usePostStore.getState()
    expect(state.posts.size).toBe(3)
    expect(state.postsByTopic.get('topic1')).toEqual(['post1', 'post2'])
    expect(state.postsByTopic.get('topic2')).toEqual(['post3'])
  })

  it('addPostメソッドが正しく動作すること', () => {
    usePostStore.getState().addPost(mockPost1)
    
    const state = usePostStore.getState()
    expect(state.posts.size).toBe(1)
    expect(state.posts.get('post1')).toEqual(mockPost1)
    expect(state.postsByTopic.get('topic1')).toEqual(['post1'])
  })

  it('updatePostメソッドが正しく動作すること', () => {
    usePostStore.setState({
      posts: new Map([['post1', mockPost1]]),
    })

    usePostStore.getState().updatePost('post1', { content: '更新された内容' })
    
    const state = usePostStore.getState()
    expect(state.posts.get('post1')?.content).toBe('更新された内容')
  })

  it('removePostメソッドが正しく動作すること', () => {
    usePostStore.setState({
      posts: new Map([['post1', mockPost1], ['post2', mockPost2]]),
      postsByTopic: new Map([['topic1', ['post1', 'post2']]]),
    })

    usePostStore.getState().removePost('post1')
    
    const state = usePostStore.getState()
    expect(state.posts.size).toBe(1)
    expect(state.posts.has('post1')).toBe(false)
    expect(state.postsByTopic.get('topic1')).toEqual(['post2'])
  })

  it('addReplyメソッドが正しく動作すること', () => {
    const reply: Post = {
      id: 'reply1',
      pubkey: 'npub999',
      content: '返信テスト',
      topicId: 'topic1',
      created_at: Date.now(),
      tags: [],
    }

    usePostStore.setState({
      posts: new Map([['post1', mockPost1]]),
    })

    usePostStore.getState().addReply('post1', reply)
    
    const state = usePostStore.getState()
    const parentPost = state.posts.get('post1')
    expect(parentPost?.replies).toHaveLength(1)
    expect(parentPost?.replies?.[0]).toEqual(reply)
  })

  it('getPostsByTopicメソッドが正しく動作すること', () => {
    usePostStore.setState({
      posts: new Map([
        ['post1', mockPost1],
        ['post2', mockPost2],
        ['post3', mockPost3],
      ]),
      postsByTopic: new Map([
        ['topic1', ['post1', 'post2']],
        ['topic2', ['post3']],
      ]),
    })

    const topic1Posts = usePostStore.getState().getPostsByTopic('topic1')
    expect(topic1Posts).toHaveLength(2)
    expect(topic1Posts[0].id).toBe('post1') // 新しい順
    expect(topic1Posts[1].id).toBe('post2')

    const topic2Posts = usePostStore.getState().getPostsByTopic('topic2')
    expect(topic2Posts).toHaveLength(1)
    expect(topic2Posts[0].id).toBe('post3')

    const emptyPosts = usePostStore.getState().getPostsByTopic('nonexistent')
    expect(emptyPosts).toHaveLength(0)
  })
})