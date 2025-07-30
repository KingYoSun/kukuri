import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { TopicsPage } from '../topics';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { Topic } from '@/stores';

// useTopicsフックのモック
const mockTopicsData: Topic[] = [
  {
    id: 'topic-1',
    name: 'テクノロジー',
    description: '技術全般について議論するトピック',
    tags: ['tech', 'programming'],
    memberCount: 150,
    postCount: 500,
    lastActive: Date.now() / 1000,
    isActive: true,
    createdAt: new Date(),
  },
  {
    id: 'topic-2',
    name: 'Nostr',
    description: 'Nostrプロトコルについて',
    tags: ['nostr', 'decentralized'],
    memberCount: 80,
    postCount: 200,
    lastActive: Date.now() / 1000,
    isActive: true,
    createdAt: new Date(),
  },
  {
    id: 'topic-3',
    name: 'P2P',
    description: 'P2P技術とネットワーキング',
    tags: ['p2p', 'networking'],
    memberCount: 45,
    postCount: 120,
    lastActive: Date.now() / 1000,
    isActive: true,
    createdAt: new Date(),
  },
];

const mockUseTopics = {
  data: mockTopicsData,
  isLoading: false,
  error: null,
};

vi.mock('@/hooks', () => ({
  useTopics: () => mockUseTopics,
}));

// TopicCardコンポーネントのモック
vi.mock('@/components/topics/TopicCard', () => ({
  TopicCard: ({ topic }: { topic: Topic }) => (
    <div data-testid={`topic-card-${topic.id}`}>
      <h3>{topic.name}</h3>
      <p>{topic.description}</p>
    </div>
  ),
}));

describe('Topics Page', () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  beforeEach(() => {
    vi.clearAllMocks();
    // デフォルトの状態に戻す
    mockUseTopics.data = mockTopicsData;
    mockUseTopics.isLoading = false;
    mockUseTopics.error = null;
  });

  function renderTopicsPage() {
    return render(
      <QueryClientProvider client={queryClient}>
        <TopicsPage />
      </QueryClientProvider>,
    );
  }

  it('ページタイトルが表示される', () => {
    renderTopicsPage();
    expect(screen.getByText('トピック一覧')).toBeInTheDocument();
  });

  it('検索入力フィールドが表示される', () => {
    renderTopicsPage();
    expect(screen.getByPlaceholderText('トピックを検索...')).toBeInTheDocument();
  });

  it('新規トピックボタンが表示される', () => {
    renderTopicsPage();
    expect(screen.getByText('新しいトピック')).toBeInTheDocument();
  });

  it('トピック一覧が表示される', () => {
    renderTopicsPage();

    mockTopicsData.forEach((topic) => {
      expect(screen.getByTestId(`topic-card-${topic.id}`)).toBeInTheDocument();
      expect(screen.getByText(topic.name)).toBeInTheDocument();
      expect(screen.getByText(topic.description)).toBeInTheDocument();
    });
  });

  it('ローディング中はローディング表示が出る', () => {
    mockUseTopics.isLoading = true;
    mockUseTopics.data = undefined as unknown as Topic[];

    renderTopicsPage();

    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  it('エラー時はエラーメッセージが表示される', () => {
    mockUseTopics.error = new Error('データ取得エラー');
    mockUseTopics.data = undefined as unknown as Topic[];

    renderTopicsPage();

    expect(
      screen.getByText('トピックの読み込みに失敗しました。しばらくしてから再度お試しください。'),
    ).toBeInTheDocument();
  });

  it('検索フィルタが機能する', async () => {
    renderTopicsPage();

    const searchInput = screen.getByPlaceholderText('トピックを検索...');

    // "Nostr"で検索
    fireEvent.change(searchInput, { target: { value: 'Nostr' } });

    await waitFor(() => {
      expect(screen.getByTestId('topic-card-topic-2')).toBeInTheDocument();
      expect(screen.queryByTestId('topic-card-topic-1')).not.toBeInTheDocument();
      expect(screen.queryByTestId('topic-card-topic-3')).not.toBeInTheDocument();
    });
  });

  it('検索フィルタがタグでも機能する', async () => {
    renderTopicsPage();

    const searchInput = screen.getByPlaceholderText('トピックを検索...');

    // "tech"タグで検索
    fireEvent.change(searchInput, { target: { value: 'tech' } });

    await waitFor(() => {
      expect(screen.getByTestId('topic-card-topic-1')).toBeInTheDocument();
      expect(screen.queryByTestId('topic-card-topic-2')).not.toBeInTheDocument();
      expect(screen.queryByTestId('topic-card-topic-3')).not.toBeInTheDocument();
    });
  });

  it('検索フィルタが説明文でも機能する', async () => {
    renderTopicsPage();

    const searchInput = screen.getByPlaceholderText('トピックを検索...');

    // 説明文の一部で検索
    fireEvent.change(searchInput, { target: { value: 'プロトコル' } });

    await waitFor(() => {
      expect(screen.getByTestId('topic-card-topic-2')).toBeInTheDocument();
      expect(screen.queryByTestId('topic-card-topic-1')).not.toBeInTheDocument();
      expect(screen.queryByTestId('topic-card-topic-3')).not.toBeInTheDocument();
    });
  });

  it('検索結果が0件の場合メッセージが表示される', async () => {
    renderTopicsPage();

    const searchInput = screen.getByPlaceholderText('トピックを検索...');

    // 存在しない検索ワード
    fireEvent.change(searchInput, { target: { value: '存在しないトピック' } });

    await waitFor(() => {
      expect(screen.getByText('検索条件に一致するトピックが見つかりません')).toBeInTheDocument();
    });
  });

  it('トピックが0件の場合メッセージが表示される', () => {
    mockUseTopics.data = [];

    renderTopicsPage();

    expect(
      screen.getByText('トピックがまだありません。最初のトピックを作成してみましょう！'),
    ).toBeInTheDocument();
  });

  it('検索フィルタは大文字小文字を区別しない', async () => {
    renderTopicsPage();

    const searchInput = screen.getByPlaceholderText('トピックを検索...');

    // 大文字で検索
    fireEvent.change(searchInput, { target: { value: 'NOSTR' } });

    await waitFor(() => {
      expect(screen.getByTestId('topic-card-topic-2')).toBeInTheDocument();
    });
  });
});
