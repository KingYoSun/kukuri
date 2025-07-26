import { describe, it, expect, beforeEach } from 'vitest'
import { usePostStore } from '../postStore'
import type { Post } from '../types'

describe('postStore', () => {
  const mockUser1 = {
    id: 'user1',
    pubkey: 'pubkey123',
    npub: 'npub123',
    name: 'ユーザー1',
    displayName: 'ユーザー1',
    picture: '',
    about: '',
    nip05: ''
  }

  const mockUser2 = {
    id: 'user2',
    pubkey: 'pubkey456',
    npub: 'npub456',
    name: 'ユーザー2',
    displayName: 'ユーザー2',
    picture: '',
    about: '',
    nip05: ''
  }

  const mockUser3 = {
    id: 'user3',
    pubkey: 'pubkey789',
    npub: 'npub789',
    name: 'ユーザー3',
    displayName: 'ユーザー3',
    picture: '',
    about: '',
    nip05: ''
  }

  const mockPost1: Post = {
    id: 'post1',
    content: 'テスト投稿1',
    author: mockUser1,
    topicId: 'topic1',
    created_at: Date.now(),
    tags: [],
    likes: 0,
    replies: []
  }

  const mockPost2: Post = {
    id: 'post2',
    content: 'テスト投稿2',
    author: mockUser2,
    topicId: 'topic1',
    created_at: Date.now() - 1000,
    tags: [],
    likes: 5,
    replies: []
  }

  const mockPost3: Post = {
    id: 'post3',
    content: 'テスト投稿3',
    author: mockUser3,
    topicId: 'topic2',
    created_at: Date.now() - 2000,
    tags: [],
    likes: 10,
    replies: []
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
    const mockReplyUser = {
      id: 'user999',
      pubkey: 'pubkey999',
      npub: 'npub999',
      name: 'リプライユーザー',
      displayName: 'リプライユーザー',
      picture: '',
      about: '',
      nip05: ''
    }
    
    const reply: Post = {
      id: 'reply1',
      content: '返信テスト',
      author: mockReplyUser,
      topicId: 'topic1',
      created_at: Date.now(),
      tags: [],
      likes: 0,
      replies: []
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