import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MainLayout } from '../MainLayout';
import { useAuthStore, useTopicStore, useUIStore } from '@/stores';

// モック
vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(() => vi.fn()),
}));

// P2P APIのモック
vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    initialize: vi.fn().mockResolvedValue(undefined),
    getNodeAddress: vi.fn().mockResolvedValue(['/ip4/127.0.0.1/tcp/4001']),
    getStatus: vi.fn().mockResolvedValue({
      connected: true,
      endpoint_id: 'test-node',
      active_topics: [],
      peer_count: 0,
    }),
    joinTopic: vi.fn().mockResolvedValue(undefined),
    leaveTopic: vi.fn().mockResolvedValue(undefined),
    broadcast: vi.fn().mockResolvedValue(undefined),
  },
}));

// useP2Pフックのモック
vi.mock('@/hooks/useP2P', () => ({
  useP2P: vi.fn(() => ({
    getTopicMessages: vi.fn(() => []),
  })),
}));

// コンポーネントのモック
vi.mock('@/components/RelayStatus', () => ({
  RelayStatus: () => <div>Relay Status</div>,
}));

vi.mock('@/components/P2PStatus', () => ({
  P2PStatus: () => <div>P2P Status</div>,
}));

describe('MainLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // デフォルトのストア状態
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
      logout: vi.fn(),
    });

    useTopicStore.setState({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
      toggleSidebar: vi.fn(),
    });
  });
  it('レイアウトが正しくレンダリングされること', () => {
    render(
      <MainLayout>
        <div data-testid="test-content">テストコンテンツ</div>
      </MainLayout>,
    );

    // ヘッダーが存在すること
    expect(screen.getByRole('banner')).toBeInTheDocument();

    // サイドバーが存在すること
    expect(screen.getByRole('complementary')).toBeInTheDocument();

    // メインコンテンツエリアが存在すること
    expect(screen.getByRole('main')).toBeInTheDocument();

    // 子要素が正しくレンダリングされること
    expect(screen.getByTestId('test-content')).toBeInTheDocument();
    expect(screen.getByText('テストコンテンツ')).toBeInTheDocument();
  });

  it('レスポンシブなレイアウト構造を持つこと', () => {
    const { container } = render(
      <MainLayout>
        <div>コンテンツ</div>
      </MainLayout>,
    );

    // フレックスボックスレイアウトの確認
    const rootDiv = container.firstChild as HTMLElement;
    expect(rootDiv).toHaveClass('h-screen', 'flex', 'flex-col');

    // メインコンテンツエリアのスクロール設定
    const mainElement = screen.getByRole('main');
    expect(mainElement).toHaveClass('flex-1', 'overflow-auto');
  });
});
