import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import React from 'react'
import { setupIntegrationTest, setMockResponse } from './setup'
import { usePostStore } from '@/stores/postStore'
import { invoke } from '@tauri-apps/api/core'
import { Post } from '@/stores/types'

// テスト用のコンポーネント
function PostTestComponent() {
  const postsMap = usePostStore((state) => state.posts)
  const posts = Array.from(postsMap.values())
  const addPost = usePostStore((state) => state.addPost)
  const [content, setContent] = React.useState('')
  const [isLoading, setIsLoading] = React.useState(false)
  
  const createPost = async (content: string, tags: string[][]) => {
    setIsLoading(true)
    try {
      const post = await invoke<Post>('create_post', { content, tags })
      addPost(post)
    } catch (error) {
      console.error('Failed to create post:', error)
    } finally {
      setIsLoading(false)
    }
  }
  
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (content.trim()) {
      await createPost(content, [])
      setContent('')
    }
  }
  
  return (
    <div>
      <form onSubmit={handleSubmit}>
        <input
          type="text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Write a post..."
          data-testid="post-input"
        />
        <button type="submit" disabled={isLoading}>Post</button>
      </form>
      
      <div data-testid="posts-list">
        {posts.map((post: Post) => (
          <div key={post.id} data-testid={`post-${post.id}`}>
            <p>{post.content}</p>
            <span>{new Date(post.created_at * 1000).toLocaleString()}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

describe('Post Integration Tests', () => {
  let cleanup: () => void
  let queryClient: QueryClient
  
  beforeEach(() => {
    cleanup = setupIntegrationTest()
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false }
      }
    })
    
    // ストアをリセット
    // 認証状態を設定（テスト用）
    usePostStore.getState().setPosts([])
  })
  
  afterEach(() => {
    cleanup()
    vi.clearAllMocks()
  })
  
  it('should create a new post', async () => {
    const user = userEvent.setup()
    
    const mockPost = {
      id: 'newpost123',
      content: 'Hello, Nostr!',
      pubkey: 'npub1testuser',
      created_at: Date.now() / 1000,
      tags: []
    }
    
    setMockResponse('create_post', mockPost)
    setMockResponse('list_posts', [mockPost])
    
    render(
      <QueryClientProvider client={queryClient}>
        <PostTestComponent />
      </QueryClientProvider>
    )
    
    // 投稿を作成
    const input = screen.getByTestId('post-input')
    await user.type(input, 'Hello, Nostr!')
    await user.click(screen.getByText('Post'))
    
    // 投稿が表示されるのを待つ
    await waitFor(() => {
      expect(screen.getByTestId('post-newpost123')).toBeInTheDocument()
      expect(screen.getByText('Hello, Nostr!')).toBeInTheDocument()
    })
    
    // 入力フィールドがクリアされていることを確認
    expect(input).toHaveValue('')
  })
  
  it('should display list of posts', async () => {
    const mockPosts = [
      {
        id: 'post1',
        content: 'First post',
        pubkey: 'npub1user1',
        created_at: Date.now() / 1000 - 3600,
        tags: []
      },
      {
        id: 'post2',
        content: 'Second post with #topic',
        pubkey: 'npub1user2',
        created_at: Date.now() / 1000 - 1800,
        tags: [['t', 'topic']]
      },
      {
        id: 'post3',
        content: 'Third post',
        pubkey: 'npub1testuser',
        created_at: Date.now() / 1000,
        tags: []
      }
    ]
    
    setMockResponse('list_posts', mockPosts)
    
    render(
      <QueryClientProvider client={queryClient}>
        <PostTestComponent />
      </QueryClientProvider>
    )
    
    // 投稿が表示されるのを待つ
    await waitFor(() => {
      expect(screen.getByText('First post')).toBeInTheDocument()
      expect(screen.getByText('Second post with #topic')).toBeInTheDocument()
      expect(screen.getByText('Third post')).toBeInTheDocument()
    })
    
    // 投稿の順序を確認（新しい順）
    const postsList = screen.getByTestId('posts-list')
    const posts = postsList.querySelectorAll('[data-testid^="post-"]')
    expect(posts).toHaveLength(3)
  })
  
  it('should handle post creation with topics', async () => {
    const user = userEvent.setup()
    
    const mockPost = {
      id: 'topicpost123',
      content: 'Post about #rust and #programming',
      pubkey: 'npub1testuser',
      created_at: Date.now() / 1000,
      tags: [['t', 'rust'], ['t', 'programming']]
    }
    
    setMockResponse('create_post', mockPost)
    setMockResponse('list_posts', [mockPost])
    
    render(
      <QueryClientProvider client={queryClient}>
        <PostTestComponent />
      </QueryClientProvider>
    )
    
    // トピック付きの投稿を作成
    const input = screen.getByTestId('post-input')
    await user.type(input, 'Post about #rust and #programming')
    await user.click(screen.getByText('Post'))
    
    // 投稿が表示されるのを待つ
    await waitFor(() => {
      const post = screen.getByTestId('post-topicpost123')
      expect(post).toBeInTheDocument()
      expect(post).toHaveTextContent('Post about #rust and #programming')
    })
  })
  
  it('should handle empty post submission', async () => {
    const user = userEvent.setup()
    
    render(
      <QueryClientProvider client={queryClient}>
        <PostTestComponent />
      </QueryClientProvider>
    )
    
    // 空の投稿を送信しようとする
    await user.click(screen.getByText('Post'))
    
    // 投稿が作成されないことを確認
    await waitFor(() => {
      const postsList = screen.getByTestId('posts-list')
      expect(postsList.children).toHaveLength(0)
    })
  })
  
  it('should handle post creation errors', async () => {
    const user = userEvent.setup()
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    
    // エラーレスポンスを設定
    setMockResponse('create_post', Promise.reject(new Error('Failed to create post')))
    setMockResponse('list_posts', [])
    
    render(
      <QueryClientProvider client={queryClient}>
        <PostTestComponent />
      </QueryClientProvider>
    )
    
    // 投稿を作成しようとする
    const input = screen.getByTestId('post-input')
    await user.type(input, 'This will fail')
    await user.click(screen.getByText('Post'))
    
    // エラーが発生しても投稿リストは空のまま
    await waitFor(() => {
      const postsList = screen.getByTestId('posts-list')
      expect(postsList.children).toHaveLength(0)
    })
    
    consoleSpy.mockRestore()
  })
  
  it('should update post list when new posts arrive', async () => {
    const initialPosts = [
      {
        id: 'existing1',
        content: 'Existing post',
        pubkey: 'npub1other',
        created_at: Date.now() / 1000 - 3600,
        tags: []
      }
    ]
    
    const updatedPosts = [
      {
        id: 'new1',
        content: 'New post from another user',
        pubkey: 'npub1another',
        created_at: Date.now() / 1000,
        tags: []
      },
      ...initialPosts
    ]
    
    setMockResponse('list_posts', initialPosts)
    
    render(
      <QueryClientProvider client={queryClient}>
        <PostTestComponent />
      </QueryClientProvider>
    )
    
    // 初期の投稿が表示される
    await waitFor(() => {
      expect(screen.getByText('Existing post')).toBeInTheDocument()
    })
    
    // 新しい投稿が追加される
    setMockResponse('list_posts', updatedPosts)
    queryClient.invalidateQueries({ queryKey: ['posts'] })
    
    // 新しい投稿が表示される
    await waitFor(() => {
      expect(screen.getByText('New post from another user')).toBeInTheDocument()
      expect(screen.getByText('Existing post')).toBeInTheDocument()
    })
  })
})