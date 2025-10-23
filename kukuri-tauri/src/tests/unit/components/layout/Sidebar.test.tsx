import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sidebar } from '@/components/layout/Sidebar';
import { useTopicStore } from '@/stores/topicStore';
import { useUIStore } from '@/stores/uiStore';
import { useP2P } from '@/hooks/useP2P';
import { useNavigate } from '@tanstack/react-router';
import type { Topic } from '@/stores/types';

vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(),
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

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(mockNavigate);
    useTopicStore.setState({
      topics: new Map(),
      joinedTopics: [],
      currentTopic: null,
      setCurrentTopic: vi.fn(),
      joinTopic: vi.fn(),
      leaveTopic: vi.fn(),
      fetchTopics: vi.fn(),
    });
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
      toggleSidebar: vi.fn(),
      setTheme: vi.fn(),
      setLoading: vi.fn(),
      setError: vi.fn(),
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

    render(<Sidebar />);

    expect(screen.getByRole('button', { name: '新規投稿' })).toBeInTheDocument();
    expect(screen.getByText('カテゴリー')).toBeInTheDocument();
    expect(screen.getByText('参加中のトピック')).toBeInTheDocument();
    expect(screen.getByText('Relay Status')).toBeInTheDocument();
    expect(screen.getByText('P2P Status')).toBeInTheDocument();
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

    render(<Sidebar />);

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

    render(<Sidebar />);

    const buttons = screen.getAllByRole('button', { name: /Topic/ });
    expect(buttons[0]).toHaveTextContent('Topic B');
    expect(getTopicMessages).toHaveBeenCalledWith('topic-b');
  });

  it('参加中のトピックがない場合はメッセージを表示する', () => {
    render(<Sidebar />);

    expect(screen.getByText('参加中のトピックはありません')).toBeInTheDocument();
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

    render(<Sidebar />);

    const button = screen.getByRole('button', { name: /Topic A/ });
    await user.click(button);

    expect(setCurrentTopic).toHaveBeenCalledWith(topic);
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
  });

  it('選択中のトピックはセカンダリスタイルで表示される', () => {
    const topic = buildTopic({ id: 'topic-a', name: 'Topic A' });

    useTopicStore.setState({
      topics: new Map([[topic.id, topic]]),
      joinedTopics: [topic.id],
      currentTopic: topic,
    });

    render(<Sidebar />);

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

    const { container } = render(<Sidebar />);
    const sidebar = container.querySelector('aside');
    expect(sidebar).toHaveClass('w-0');
  });
});
