import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MainLayout } from '../MainLayout'
import { useAuthStore, useTopicStore, useUIStore } from '@/stores'

// モック
vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(() => vi.fn()),
}))

describe('MainLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    
    // デフォルトのストア状態
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      logout: vi.fn(),
    })
    
    useTopicStore.setState({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
      setCurrentTopic: vi.fn(),
    })
    
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
      toggleSidebar: vi.fn(),
    })
  })
  it('レイアウトが正しくレンダリングされること', () => {
    render(
      <MainLayout>
        <div data-testid="test-content">テストコンテンツ</div>
      </MainLayout>
    )

    // ヘッダーが存在すること
    expect(screen.getByRole('banner')).toBeInTheDocument()
    
    // サイドバーが存在すること
    expect(screen.getByRole('complementary')).toBeInTheDocument()
    
    // メインコンテンツエリアが存在すること
    expect(screen.getByRole('main')).toBeInTheDocument()
    
    // 子要素が正しくレンダリングされること
    expect(screen.getByTestId('test-content')).toBeInTheDocument()
    expect(screen.getByText('テストコンテンツ')).toBeInTheDocument()
  })

  it('レスポンシブなレイアウト構造を持つこと', () => {
    const { container } = render(
      <MainLayout>
        <div>コンテンツ</div>
      </MainLayout>
    )

    // フレックスボックスレイアウトの確認
    const rootDiv = container.firstChild as HTMLElement
    expect(rootDiv).toHaveClass('h-screen', 'flex', 'flex-col')
    
    // メインコンテンツエリアのスクロール設定
    const mainElement = screen.getByRole('main')
    expect(mainElement).toHaveClass('flex-1', 'overflow-auto')
  })
})