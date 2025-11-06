import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TrendingPage } from '@/routes/trending';
import type { TrendingPostsResult, TrendingTopicsResult } from '@/hooks/useTrendingFeeds';

const trendingMocks = vi.hoisted(() => ({
  useTrendingTopicsQueryMock: vi.fn(),
  useTrendingPostsQueryMock: vi.fn(),
}));

vi.mock('@/hooks/useTrendingFeeds', () => ({
  useTrendingTopicsQuery: trendingMocks.useTrendingTopicsQueryMock,
  useTrendingPostsQuery: trendingMocks.useTrendingPostsQueryMock,
}));

const buildTopicsResult = (overrides?: Partial<TrendingTopicsResult>): TrendingTopicsResult => ({
  generatedAt: Date.now(),
  topics: [
    {
      topicId: 'topic-1',
      name: '技術トレンド',
      description: '技術に関する最新トピック',
      memberCount: 120,
      postCount: 340,
      trendingScore: 87.5,
      rank: 1,
      scoreChange: 4.2,
    },
  ],
  ...overrides,
});

const buildPostsResult = (overrides?: Partial<TrendingPostsResult>): TrendingPostsResult => ({
  generatedAt: Date.now(),
  topics: [
    {
      topicId: 'topic-1',
      topicName: '技術トレンド',
      relativeRank: 1,
      posts: [
        {
          id: 'post-1',
          content: '分散型SNSの最新動向について共有します。',
          created_at: Math.floor(Date.now() / 1000) - 60,
          topicId: 'topic-1',
          author: {
            id: 'author-1',
            pubkey: 'pubkey-1',
            npub: 'npub-1',
            name: 'Tech Writer',
            displayName: 'Tech Writer',
            picture: '',
            about: '',
            nip05: '',
            avatar: null,
          },
          tags: [],
          likes: 10,
          boosts: 2,
          replies: [],
        },
      ],
    },
  ],
  ...overrides,
});

function renderTrendingPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TrendingPage />
    </QueryClientProvider>,
  );
}

describe('TrendingPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('ローディング中はスピナーを表示する', () => {
    trendingMocks.useTrendingTopicsQueryMock.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: true,
    });
    trendingMocks.useTrendingPostsQueryMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });

    renderTrendingPage();

    expect(screen.getByTestId('trending-loading')).toBeInTheDocument();
  });

  it('トレンドトピックと投稿プレビューを表示する', () => {
    const topicsResult = buildTopicsResult();
    const postsResult = buildPostsResult();

    trendingMocks.useTrendingTopicsQueryMock.mockReturnValue({
      data: topicsResult,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });
    trendingMocks.useTrendingPostsQueryMock.mockReturnValue({
      data: postsResult,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });

    renderTrendingPage();

    expect(screen.getByText('トレンド')).toBeInTheDocument();
    expect(screen.getByTestId('trending-topic-topic-1')).toBeInTheDocument();
    expect(screen.getByText('技術トレンド')).toBeInTheDocument();
    expect(screen.getByText('スコア')).toBeInTheDocument();
    expect(screen.getByTestId('trending-topic-topic-1-posts')).toBeInTheDocument();
    expect(screen.getByText('分散型SNSの最新動向について共有します。')).toBeInTheDocument();
  });

  it('トレンドが空の場合は案内カードを表示する', () => {
    trendingMocks.useTrendingTopicsQueryMock.mockReturnValue({
      data: buildTopicsResult({ topics: [] }),
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });
    trendingMocks.useTrendingPostsQueryMock.mockReturnValue({
      data: buildPostsResult({ topics: [] }),
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });

    renderTrendingPage();

    expect(screen.getByTestId('trending-empty')).toBeInTheDocument();
    expect(screen.getByText('トレンドはまだありません')).toBeInTheDocument();
  });

  it('トレンド取得に失敗した場合はエラーメッセージと再試行ボタンを表示する', async () => {
    const refetchMock = vi.fn();

    trendingMocks.useTrendingTopicsQueryMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: new Error('取得に失敗しました'),
      refetch: refetchMock,
      isFetching: false,
    });
    trendingMocks.useTrendingPostsQueryMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });

    const user = userEvent.setup();
    renderTrendingPage();

    expect(screen.getByTestId('trending-error')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '再試行' }));
    expect(refetchMock).toHaveBeenCalled();
  });

  it('投稿プレビュー取得に失敗した場合は警告を表示する', async () => {
    trendingMocks.useTrendingTopicsQueryMock.mockReturnValue({
      data: buildTopicsResult(),
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });
    const refetchPosts = vi.fn();
    trendingMocks.useTrendingPostsQueryMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: new Error('プレビュー失敗'),
      refetch: refetchPosts,
      isFetching: false,
    });

    const user = userEvent.setup();
    renderTrendingPage();

    const alert = screen.getByTestId('trending-posts-error');
    expect(alert).toBeInTheDocument();
    expect(alert).toHaveTextContent('投稿プレビューの取得に失敗しました');

    const retryButton = screen.getByRole('button', { name: '再試行' });
    await user.click(retryButton);
    expect(refetchPosts).toHaveBeenCalled();
  });

  it('Summary Panel で派生メトリクスを表示する', () => {
    const generatedAt = Date.now() - 60_000;
    trendingMocks.useTrendingTopicsQueryMock.mockReturnValue({
      data: buildTopicsResult({
        generatedAt,
        topics: [
          {
            topicId: 'topic-1',
            name: '技術トレンド',
            description: null,
            memberCount: 100,
            postCount: 200,
            trendingScore: 80,
            rank: 1,
            scoreChange: null,
          },
          {
            topicId: 'topic-2',
            name: '分散システム',
            description: null,
            memberCount: 50,
            postCount: 120,
            trendingScore: 60,
            rank: 2,
            scoreChange: -2,
          },
        ],
      }),
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });
    trendingMocks.useTrendingPostsQueryMock.mockReturnValue({
      data: buildPostsResult({
        topics: [
          {
            topicId: 'topic-1',
            topicName: '技術トレンド',
            relativeRank: 1,
            posts: [buildPostsResult().topics[0].posts[0]],
          },
          {
            topicId: 'topic-2',
            topicName: '分散システム',
            relativeRank: 2,
            posts: [
              {
                ...buildPostsResult().topics[0].posts[0],
                id: 'post-2',
              },
            ],
          },
        ],
        generatedAt,
      }),
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    });

    renderTrendingPage();

    expect(screen.getByTestId('trending-summary-panel')).toBeInTheDocument();
    expect(screen.getByTestId('trending-summary-topics')).toHaveTextContent('2件');
    expect(screen.getByTestId('trending-summary-posts')).toHaveTextContent('2件');
    expect(screen.getByTestId('trending-summary-score')).toHaveTextContent('70.0pt');
    expect(screen.getByTestId('trending-summary-updated')).toBeInTheDocument();
    expect(screen.getByTestId('trending-summary-direct-messages')).toHaveTextContent('0件');
  });
});
