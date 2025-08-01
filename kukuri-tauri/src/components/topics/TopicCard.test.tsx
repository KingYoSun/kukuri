import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { TopicCard } from './TopicCard';
import type { Topic } from '@/stores';

// zustand storeのモック
const mockJoinedTopics: string[] = [];
const mockJoinTopic = vi.fn();
const mockLeaveTopic = vi.fn();

vi.mock('@/stores', () => ({
  useTopicStore: () => ({
    joinedTopics: mockJoinedTopics,
    joinTopic: mockJoinTopic,
    leaveTopic: mockLeaveTopic,
  }),
}));

// p2p APIのモック - TopicCardでは使用しなくなったので削除
// joinTopic/leaveTopicの処理はtopicStore内で行われるようになった

// Tanstack Routerのモック
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    to,
    params,
    className,
  }: {
    children: React.ReactNode;
    to: string;
    params?: Record<string, string>;
    className?: string;
  }) => (
    <a href={to.replace('$topicId', params?.topicId || '')} className={className}>
      {children}
    </a>
  ),
}));

describe('TopicCard', () => {
  const mockTopic: Topic = {
    id: 'test-topic-1',
    name: 'テストトピック',
    description: 'これはテスト用のトピックです',
    tags: ['test', 'sample'],
    memberCount: 42,
    postCount: 123,
    lastActive: Date.now() / 1000,
    isActive: true,
    createdAt: new Date(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockJoinedTopics.length = 0;
  });

  it('トピック情報を正しく表示する', () => {
    render(<TopicCard topic={mockTopic} />);

    expect(screen.getByText(mockTopic.name)).toBeInTheDocument();
    expect(screen.getByText(mockTopic.description)).toBeInTheDocument();
    expect(screen.getByText(`${mockTopic.memberCount} メンバー`)).toBeInTheDocument();
    expect(screen.getByText(`${mockTopic.postCount} 投稿`)).toBeInTheDocument();
  });

  it('タグを正しく表示する', () => {
    render(<TopicCard topic={mockTopic} />);

    mockTopic.tags.forEach((tag) => {
      expect(screen.getByText(tag)).toBeInTheDocument();
    });
  });

  it('未参加の場合「参加」ボタンが表示される', () => {
    render(<TopicCard topic={mockTopic} />);

    const joinButton = screen.getByText('参加');
    expect(joinButton).toBeInTheDocument();
  });

  it('参加済みの場合「参加中」ボタンが表示される', () => {
    mockJoinedTopics.push(mockTopic.id);

    render(<TopicCard topic={mockTopic} />);

    const joinedButton = screen.getByText('参加中');
    expect(joinedButton).toBeInTheDocument();
  });

  it('参加ボタンをクリックするとjoinTopicが呼ばれる', async () => {
    render(<TopicCard topic={mockTopic} />);

    const joinButton = screen.getByText('参加');
    fireEvent.click(joinButton);

    await waitFor(() => {
      expect(mockJoinTopic).toHaveBeenCalledWith(mockTopic.id);
    });
    expect(mockLeaveTopic).not.toHaveBeenCalled();
  });

  it('参加ボタンにアクセシビリティ属性が設定される', () => {
    render(<TopicCard topic={mockTopic} />);

    const joinButton = screen.getByText('参加');
    expect(joinButton).toHaveAttribute('aria-pressed', 'false');
    expect(joinButton).toHaveAttribute('aria-label', `「${mockTopic.name}」に参加`);
  });

  it('参加中ボタンにアクセシビリティ属性が設定される', () => {
    mockJoinedTopics.push(mockTopic.id);

    render(<TopicCard topic={mockTopic} />);

    const joinedButton = screen.getByText('参加中');
    expect(joinedButton).toHaveAttribute('aria-pressed', 'true');
    expect(joinedButton).toHaveAttribute('aria-label', `「${mockTopic.name}」から離脱`);
  });

  it('ローディング中はボタンが無効化される', async () => {
    // joinTopicを遅延させる
    mockJoinTopic.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));

    render(<TopicCard topic={mockTopic} />);

    const joinButton = screen.getByText('参加');
    fireEvent.click(joinButton);

    // ローディング中の確認
    await waitFor(() => {
      expect(joinButton).toBeDisabled();
      expect(screen.getByRole('button')).toHaveTextContent('参加');
    });

    // 完了後の確認
    await waitFor(() => {
      expect(joinButton).not.toBeDisabled();
    });
  });

  it('エラー時はトーストが表示される', async () => {
    // joinTopicがエラーを投げるようにモック
    mockJoinTopic.mockRejectedValueOnce(new Error('Network error'));

    render(<TopicCard topic={mockTopic} />);

    const joinButton = screen.getByText('参加');
    fireEvent.click(joinButton);

    // エラー処理の確認
    await waitFor(() => {
      expect(mockJoinTopic).toHaveBeenCalledWith(mockTopic.id);
    });
  });

  it('参加中ボタンをクリックするとleaveTopicが呼ばれる', async () => {
    mockJoinedTopics.push(mockTopic.id);

    render(<TopicCard topic={mockTopic} />);

    const joinedButton = screen.getByText('参加中');
    fireEvent.click(joinedButton);

    await waitFor(() => {
      expect(mockLeaveTopic).toHaveBeenCalledWith(mockTopic.id);
    });
    expect(mockJoinTopic).not.toHaveBeenCalled();
  });

  it('最終アクティブ時間を日本語で表示する', () => {
    render(<TopicCard topic={mockTopic} />);

    // 相対時間なので正確な文字列は確認できないが、
    // "前"という文字が含まれることを確認
    const timeElements = screen.getAllByText(/前/);
    expect(timeElements.length).toBeGreaterThan(0);
  });

  it('lastActiveがない場合「活動なし」と表示される', () => {
    const inactiveTopic = {
      ...mockTopic,
      lastActive: undefined,
    };

    render(<TopicCard topic={inactiveTopic} />);

    expect(screen.getByText('活動なし')).toBeInTheDocument();
  });

  it('トピック名のリンクが正しく設定される', () => {
    render(<TopicCard topic={mockTopic} />);

    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', `/topics/${mockTopic.id}`);
  });
});
