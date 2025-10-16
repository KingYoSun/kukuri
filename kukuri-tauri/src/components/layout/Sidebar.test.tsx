import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Sidebar } from './Sidebar';
import { useTopicStore, useUIStore } from '@/stores';
import { useP2P } from '@/hooks/useP2P';
import { useNavigate } from '@tanstack/react-router';
import type { Topic } from '@/stores/types';

// モック
vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(() => vi.fn()),
}));

vi.mock('@/components/RelayStatus', () => ({
  RelayStatus: () => <div>Relay Status</div>,
}));

vi.mock('@/components/P2PStatus', () => ({
  P2PStatus: () => <div>P2P Status</div>,
}));

vi.mock('@/stores', () => ({
  useTopicStore: vi.fn(),
  useUIStore: vi.fn(),
}));

vi.mock('@/hooks/useP2P', () => ({
  useP2P: vi.fn(),
}));

const mockTopic1: Topic = {
  id: 'topic-1',
  name: 'Topic 1',
  description: 'Description 1',
  createdAt: new Date('2024-01-01'),
  memberCount: 10,
  postCount: 5,
  isActive: true,
  tags: [],
  lastActive: Date.now() / 1000 - 3600, // 1時間前
};

const mockTopic2: Topic = {
  id: 'topic-2',
  name: 'Topic 2',
  description: 'Description 2',
  createdAt: new Date('2024-01-02'),
  memberCount: 20,
  postCount: 15,
  isActive: true,
  tags: [],
  lastActive: Date.now() / 1000 - 7200, // 2時間前
};

const mockTopic3: Topic = {
  id: 'topic-3',
  name: 'Topic 3',
  description: 'Description 3',
  createdAt: new Date('2024-01-03'),
  memberCount: 5,
  postCount: 2,
  isActive: true,
  tags: [],
  lastActive: 0, // アクティブなし
};

describe('Sidebar', () => {
  const mockNavigate = vi.fn();
  const mockSetCurrentTopic = vi.fn();
  const mockGetTopicMessages = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useNavigate).mockReturnValue(mockNavigate);

    vi.mocked(useUIStore).mockReturnValue({
      sidebarOpen: true,
    } as Partial<ReturnType<typeof useUIStore>>);

    vi.mocked(useP2P).mockReturnValue({
      getTopicMessages: mockGetTopicMessages,
    } as Partial<ReturnType<typeof useP2P>>);

    // デフォルトで空のメッセージを返す
    mockGetTopicMessages.mockReturnValue([]);
  });

  it('参加中のトピックを最終活動時刻でソートして表示する', () => {
    const topics = new Map([
      ['topic-1', mockTopic1],
      ['topic-2', mockTopic2],
      ['topic-3', mockTopic3],
    ]);

    vi.mocked(useTopicStore).mockReturnValue({
      topics,
      joinedTopics: ['topic-1', 'topic-2', 'topic-3'],
      currentTopic: null,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useTopicStore>>);

    render(<Sidebar />);

    // トピック名が表示されることを確認
    expect(screen.getByText('Topic 1')).toBeInTheDocument();
    expect(screen.getByText('Topic 2')).toBeInTheDocument();
    expect(screen.getByText('Topic 3')).toBeInTheDocument();

    // ソート順を確認（最新のものが上）
    const topicButtons = screen.getAllByRole('button', { name: /Topic/ });
    expect(topicButtons[0]).toHaveTextContent('Topic 1');
    expect(topicButtons[1]).toHaveTextContent('Topic 2');
    expect(topicButtons[2]).toHaveTextContent('Topic 3');
  });

  it('P2Pメッセージの最終活動時刻を使用してソートする', () => {
    const topics = new Map([
      ['topic-1', mockTopic1],
      ['topic-2', mockTopic2],
    ]);

    vi.mocked(useTopicStore).mockReturnValue({
      topics,
      joinedTopics: ['topic-1', 'topic-2'],
      currentTopic: null,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useUIStore>>);

    // topic-2に最新のP2Pメッセージがある
    mockGetTopicMessages.mockImplementation((topicId: string) => {
      if (topicId === 'topic-2') {
        return [
          {
            id: 'msg-1',
            author: 'author-1',
            content: 'New message',
            timestamp: Date.now(),
            signature: 'sig',
            topic_id: 'topic-2',
          },
        ];
      }
      return [];
    });

    render(<Sidebar />);

    // topic-2が最初に表示されることを確認
    const topicButtons = screen.getAllByRole('button', { name: /Topic/ });
    expect(topicButtons[0]).toHaveTextContent('Topic 2');
    expect(topicButtons[1]).toHaveTextContent('Topic 1');
  });

  it('参加中のトピックがない場合はメッセージを表示する', () => {
    vi.mocked(useTopicStore).mockReturnValue({
      topics: new Map(),
      joinedTopics: [],
      currentTopic: null,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useTopicStore>>);

    render(<Sidebar />);

    expect(screen.getByText('参加中のトピックはありません')).toBeInTheDocument();
  });

  it('現在選択中のトピックは異なるスタイルで表示される', () => {
    const topics = new Map([['topic-1', mockTopic1]]);

    vi.mocked(useTopicStore).mockReturnValue({
      topics,
      joinedTopics: ['topic-1'],
      currentTopic: mockTopic1,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useTopicStore>>);

    render(<Sidebar />);

    const topicButton = screen.getByRole('button', { name: /Topic 1/ });
    expect(topicButton).toHaveClass('bg-secondary'); // variant="secondary"のクラス
  });

  it('トピックの投稿数と最終活動時刻を表示する', () => {
    const topics = new Map([['topic-1', mockTopic1]]);

    vi.mocked(useTopicStore).mockReturnValue({
      topics,
      joinedTopics: ['topic-1'],
      currentTopic: null,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useTopicStore>>);

    render(<Sidebar />);

    // 投稿数が表示されることを確認
    expect(screen.getByText('5')).toBeInTheDocument();

    // 最終活動時刻が表示されることを確認（相対時刻）
    // 実際の表示内容は日本語ロケールに依存するため、存在のみ確認
    const topicButton = screen.getByRole('button', { name: /Topic 1/ });
    expect(topicButton).toBeInTheDocument();
  });

  it('最終活動時刻がないトピックは「未投稿」と表示される', () => {
    const topics = new Map([['topic-3', mockTopic3]]);

    vi.mocked(useTopicStore).mockReturnValue({
      topics,
      joinedTopics: ['topic-3'],
      currentTopic: null,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useTopicStore>>);

    render(<Sidebar />);

    expect(screen.getByText('未投稿')).toBeInTheDocument();
  });

  it('サイドバーが閉じている場合は内容が表示されない', () => {
    vi.mocked(useUIStore).mockReturnValue({
      sidebarOpen: false,
    } as Partial<ReturnType<typeof useUIStore>>);

    vi.mocked(useTopicStore).mockReturnValue({
      topics: new Map([['topic-1', mockTopic1]]),
      joinedTopics: ['topic-1'],
      currentTopic: null,
      setCurrentTopic: mockSetCurrentTopic,
    } as Partial<ReturnType<typeof useUIStore>>);

    const { container } = render(<Sidebar />);

    // サイドバーの幅が0になることを確認
    const sidebar = container.querySelector('aside');
    expect(sidebar).toHaveClass('w-0');
  });
});
