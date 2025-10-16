import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sidebar } from '../Sidebar';
import { useTopicStore, useUIStore } from '@/stores';
import { useNavigate } from '@tanstack/react-router';
import type { Topic } from '@/stores';

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

describe('Sidebar', () => {
  const mockNavigate = vi.fn();
  const mockTopic1: Topic = {
    id: 'topic1',
    name: 'technology',
    description: '技術関連のトピック',
    memberCount: 1234,
    postCount: 567,
    tags: [],
    lastActive: Date.now(),
    isActive: true,
    createdAt: new Date(),
  };
  const mockTopic2: Topic = {
    id: 'topic2',
    name: 'nostr',
    description: 'Nostr関連のトピック',
    memberCount: 456,
    postCount: 234,
    tags: [],
    lastActive: Date.now(),
    isActive: true,
    createdAt: new Date(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(mockNavigate);
  });
  it('サイドバーの基本要素が表示されること', () => {
    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    render(<Sidebar />);

    // 新規投稿ボタンが表示されること
    expect(screen.getByRole('button', { name: /新規投稿/i })).toBeInTheDocument();

    // カテゴリーセクションが表示されること
    expect(screen.getByText('カテゴリー')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /トレンド/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /フォロー中/i })).toBeInTheDocument();

    // トピックセクションが表示されること
    expect(screen.getByText('参加中のトピック')).toBeInTheDocument();
  });

  it('トピックリストが正しく表示されること', () => {
    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    render(<Sidebar />);

    // 各トピックが表示されること
    expect(screen.getByRole('button', { name: /technology/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /nostr/i })).toBeInTheDocument();

    // トピックの投稿数が表示されること
    expect(screen.getByText('567')).toBeInTheDocument();
    expect(screen.getByText('234')).toBeInTheDocument();
  });

  it('参加中のトピックがない場合の表示', () => {
    // 空のトピックリストを設定
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
    });

    render(<Sidebar />);

    expect(screen.getByText('参加中のトピックはありません')).toBeInTheDocument();
  });

  it('新規投稿ボタンがクリック可能であること', async () => {
    const user = userEvent.setup();

    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    render(<Sidebar />);

    const newPostButton = screen.getByRole('button', { name: /新規投稿/i });

    // ボタンがクリック可能であることを確認
    await user.click(newPostButton);
    expect(newPostButton).toBeEnabled();
  });

  it('カテゴリーボタンがクリック可能であること', async () => {
    const user = userEvent.setup();

    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    render(<Sidebar />);

    const trendButton = screen.getByRole('button', { name: /トレンド/i });
    const followingButton = screen.getByRole('button', { name: /フォロー中/i });

    // 各ボタンがクリック可能であることを確認
    await user.click(trendButton);
    expect(trendButton).toBeEnabled();

    await user.click(followingButton);
    expect(followingButton).toBeEnabled();
  });

  it('トピックボタンクリックでナビゲートされること', async () => {
    const user = userEvent.setup();
    const setCurrentTopic = vi.fn();

    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic,
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    render(<Sidebar />);

    const technologyButton = screen.getByRole('button', { name: /technology/i });
    await user.click(technologyButton);

    expect(setCurrentTopic).toHaveBeenCalledWith(mockTopic1);
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
  });

  it('現在のトピックがハイライトされること', () => {
    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: mockTopic1,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    render(<Sidebar />);

    // 現在のトピックのボタンを取得
    const technologyButton = screen.getByRole('button', { name: /technology/i });
    const nostrButton = screen.getByRole('button', { name: /nostr/i });

    // 現在のトピック（technology）はsecondaryスタイルを持つ
    // shadcn/uiのButtonコンポーネントのsecondaryバリアントのクラスを確認
    expect(technologyButton.className).toContain('bg-secondary');

    // 他のトピックはghostスタイルを持つ
    expect(nostrButton.className).not.toContain('bg-secondary');
  });

  it('サイドバーの開閉状態が反映されること', () => {
    // 必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    const { rerender } = render(<Sidebar />);

    // 開いている状態
    expect(screen.getByRole('complementary')).toHaveClass('w-64');

    // 閉じている状態に変更
    act(() => {
      useUIStore.setState({ sidebarOpen: false });
    });

    rerender(<Sidebar />);
    expect(screen.getByRole('complementary')).toHaveClass('w-0');
  });

  it('適切なスタイリングとレイアウトが適用されていること', () => {
    // テストに必要なストア状態を設定
    useTopicStore.setState({
      topics: new Map([
        ['topic1', mockTopic1],
        ['topic2', mockTopic2],
      ]),
      currentTopic: null,
      joinedTopics: ['topic1', 'topic2'],
      setCurrentTopic: vi.fn(),
    });

    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });

    const { container } = render(<Sidebar />);

    const aside = container.querySelector('aside');
    expect(aside).toHaveClass('border-r', 'bg-background');

    // ScrollAreaが存在することを確認
    expect(container.querySelector('[data-radix-scroll-area-viewport]')).toBeInTheDocument();
  });
});
