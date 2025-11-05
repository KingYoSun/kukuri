import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sidebar } from '@/components/layout/Sidebar';
import { useTopicStore } from '@/stores/topicStore';
import { useUIStore } from '@/stores/uiStore';
import { useUIStore as useUIStoreFromIndex } from '@/stores';
import { useP2P } from '@/hooks/useP2P';
import { useNavigate, useLocation } from '@tanstack/react-router';
import {
  prefetchTrendingCategory,
  prefetchFollowingCategory,
} from '@/hooks/useTrendingFeeds';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { Topic } from '@/stores/types';
import { useComposerStore } from '@/stores/composerStore';

vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(),
  useLocation: vi.fn(),
}));

vi.mock('@/components/RelayStatus', () => ({
  RelayStatus: () => <div>Relay Status</div>,
}));

vi.mock('@/components/P2PStatus', () => ({
  P2PStatus: () => <div>P2P Status</div>,
}));

vi.mock('@/hooks/useP2P', () => ({
  useP2P: vi.fn(() => ({
    getTopicMessages: vi.fn(() => []),
  })),
}));

vi.mock('@/hooks/useTrendingFeeds', () => ({
  prefetchTrendingCategory: vi.fn(),
  prefetchFollowingCategory: vi.fn(),
}));

const buildTopic = (overrides: Partial<Topic>): Topic => ({
  id: 'topic-1',
  name: 'Topic 1',
  description: 'Description',
  createdAt: new Date('2024-01-01'),
  memberCount: 0,
  postCount: 0,
  isActive: true,
  tags: [],
  lastActive: Date.now() / 1000,
  ...overrides,
});

describe('Sidebar', () => {
  const mockNavigate = vi.fn();
  const renderSidebar = () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    return render(
      <QueryClientProvider client={queryClient}>
        <Sidebar />
      </QueryClientProvider>,
    );
  };

  it('UIストアのエクスポートが同一インスタンスであること', () => {
    expect(useUIStore).toBe(useUIStoreFromIndex);
  });

  beforeEach(() => {
    vi.clearAllMocks();
    useComposerStore.getState().reset();
    vi.mocked(useNavigate).mockReturnValue(mockNavigate);
    vi.mocked(useLocation).mockReturnValue({ pathname: '/' } as { pathname: string });
    useTopicStore.setState({
      topics: new Map(),
      joinedTopics: [],
      currentTopic: null,
      topicUnreadCounts: new Map(),
      topicLastReadAt: new Map(),
      setCurrentTopic: vi.fn(),
      joinTopic: vi.fn(),
      leaveTopic: vi.fn(),
      fetchTopics: vi.fn(),
      markTopicRead: vi.fn(),
      handleIncomingTopicMessage: vi.fn(),
    });
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
      activeSidebarCategory: null,
    });
    vi.mocked(useP2P).mockReturnValue({
      getTopicMessages: vi.fn(() => []),
      joinTopic: vi.fn(),
      leaveTopic: vi.fn(),
    } as ReturnType<typeof useP2P>);
  });

  it('基本的なセクションが表示される', () => {
    const topicA = buildTopic({ id: 'topic-a', name: 'technology' });
    const topicB = buildTopic({ id: 'topic-b', name: 'nostr' });

    useTopicStore.setState({
      topics: new Map([
        [topicA.id, topicA],
        [topicB.id, topicB],
      ]),
      joinedTopics: [topicA.id, topicB.id],
    });

    renderSidebar();

    expect(screen.getByRole('button', { name: '新規投稿' })).toBeInTheDocument();
    expect(screen.getByText('カテゴリー')).toBeInTheDocument();
    expect(screen.getByText('参加中のトピック')).toBeInTheDocument();
    expect(screen.getByText('Relay Status')).toBeInTheDocument();
    expect(screen.getByText('P2P Status')).toBeInTheDocument();
  });

  it('新規投稿ボタンをクリックするとコンポーザーが開く', async () => {
    const user = userEvent.setup();
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });

    useTopicStore.setState({
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
      currentTopic: topic,
      topicUnreadCounts: new Map(),
      topicLastReadAt: new Map(),
    });

    renderSidebar();

    await user.click(screen.getByRole('button', { name: '新規投稿' }));

    const composerState = useComposerStore.getState();
    expect(composerState.isOpen).toBe(true);
    expect(composerState.topicId).toBe('topic-a');
  });

  it('最終活動時刻を考慮してトピックが降順で表示される', () => {
    const now = Math.floor(Date.now() / 1000);
    const topicA = buildTopic({
      id: 'topic-a',
      name: 'Topic A',
      lastActive: now - 60,
    });
    const topicB = buildTopic({
      id: 'topic-b',
      name: 'Topic B',
      lastActive: now - 10,
    });
    const topicC = buildTopic({
      id: 'topic-c',
      name: 'Topic C',
      lastActive: 0,
    });

    useTopicStore.setState({
      topics: new Map([
        [topicA.id, topicA],
        [topicB.id, topicB],
        [topicC.id, topicC],
      ]),
      joinedTopics: [topicA.id, topicB.id, topicC.id],
    });

    renderSidebar();

    const buttons = screen.getAllByRole('button', { name: /Topic/ });
    expect(buttons[0]).toHaveTextContent('Topic B');
    expect(buttons[1]).toHaveTextContent('Topic A');
    expect(buttons[2]).toHaveTextContent('Topic C');
  });

  it('P2Pメッセージで最新活動が更新されたトピックを最上位に表示する', () => {
    const now = Math.floor(Date.now() / 1000);
    const topicA = buildTopic({
      id: 'topic-a',
      name: 'Topic A',
      lastActive: now - 60,
    });
    const topicB = buildTopic({
      id: 'topic-b',
      name: 'Topic B',
      lastActive: now - 120,
    });

    const getTopicMessages = vi.fn((topicId: string) =>
      topicId === 'topic-b'
        ? [
            {
              id: 'message-1',
              author: 'author-1',
              content: 'Hello',
              timestamp: Date.now(),
              signature: 'sig',
              topic_id: 'topic-b',
            },
          ]
        : [],
    );

    vi.mocked(useP2P).mockReturnValue({
      getTopicMessages,
      joinTopic: vi.fn(),
      leaveTopic: vi.fn(),
    } as ReturnType<typeof useP2P>);

    useTopicStore.setState({
      topics: new Map([
        [topicA.id, topicA],
        [topicB.id, topicB],
      ]),
      joinedTopics: [topicA.id, topicB.id],
    });

    renderSidebar();

    const buttons = screen.getAllByRole('button', { name: /Topic/ });
    expect(buttons[0]).toHaveTextContent('Topic B');
    expect(getTopicMessages).toHaveBeenCalledWith('topic-b');
  });

  it('参加中のトピックがない場合はメッセージを表示する', () => {
    renderSidebar();

    expect(screen.getByText('参加中のトピックはありません')).toBeInTheDocument();
  });

  it('未読件数をバッジ表示する', () => {
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });
    useTopicStore.setState((state) => ({
      ...state,
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
      topicUnreadCounts: new Map([[topic.id, 5]]),
    }));

    renderSidebar();

    expect(screen.getByTestId('topic-topic-a-unread')).toHaveTextContent('5');
  });

  it('未読カウントが0の場合はバッジを表示しない', () => {
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });
    useTopicStore.setState((state) => ({
      ...state,
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
      topicUnreadCounts: new Map([[topic.id, 0]]),
    }));

    renderSidebar();

    expect(screen.queryByTestId('topic-topic-a-unread')).not.toBeInTheDocument();
  });

  it('最後の活動が存在しない場合は未投稿と表示する', () => {
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A', lastActive: 0 });
    useTopicStore.setState((state) => ({
      ...state,
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
    }));

    renderSidebar();

    expect(screen.getByText('未投稿')).toBeInTheDocument();
  });

  it('トピックをクリックするとナビゲーションと選択状態が更新される', async () => {
    const user = userEvent.setup();
    const setCurrentTopic = vi.fn();
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });

    useTopicStore.setState({
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
      setCurrentTopic,
    });

    renderSidebar();

    const button = screen.getByRole('button', { name: /Topic A/ });
    await user.click(button);

    expect(setCurrentTopic).toHaveBeenCalledWith(topic);
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
    expect(useUIStore.getState().activeSidebarCategory).toBeNull();
  });

  it('選択中のトピックはセカンダリスタイルで表示される', () => {
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });

    useTopicStore.setState({
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
      currentTopic: topic,
    });

    renderSidebar();

    const button = screen.getByRole('button', { name: /Topic A/ });
    expect(button.className).toContain('bg-secondary');
  });

  it('サイドバーが閉じている場合は幅が0になる', () => {
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });
    useTopicStore.setState({
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
    });

    useUIStore.setState({ sidebarOpen: false });

    const { container } = renderSidebar();
    const sidebar = container.querySelector('aside');
    expect(sidebar).toHaveClass('w-0');
  });

  it('トレンドカテゴリーをクリックするとprefetchとナビゲーションが実行される', async () => {
    const user = userEvent.setup();
    renderSidebar();

    await user.click(screen.getByTestId('category-trending'));

    expect(prefetchTrendingCategory).toHaveBeenCalledTimes(1);
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/trending' });
    expect(useUIStore.getState().activeSidebarCategory).toBe('trending');
  });

  it('フォロー中カテゴリーをクリックするとprefetchとナビゲーションが実行される', async () => {
    const user = userEvent.setup();
    renderSidebar();

    await user.click(screen.getByTestId('category-following'));

    expect(prefetchFollowingCategory).toHaveBeenCalledTimes(1);
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/following' });
    expect(useUIStore.getState().activeSidebarCategory).toBe('following');
  });

  it('現在のルートに応じてカテゴリーが強調される', async () => {
    vi.mocked(useLocation).mockReturnValue({ pathname: '/trending' } as { pathname: string });
    renderSidebar();

    await waitFor(() => expect(useUIStore.getState().activeSidebarCategory).toBe('trending'));
  });
});
