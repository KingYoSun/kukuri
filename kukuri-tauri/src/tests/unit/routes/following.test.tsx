import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { FollowingPage } from '@/routes/following';
import type { Post } from '@/stores';

const followingMocks = vi.hoisted(() => ({
  useFollowingFeedQueryMock: vi.fn(),
}));

vi.mock('@/hooks/useTrendingFeeds', () => ({
  useFollowingFeedQuery: followingMocks.useFollowingFeedQueryMock,
}));

const buildPost = (overrides?: Partial<Post>): Post => ({
  id: 'post-1',
  content: 'フォロー中ユーザーの最新投稿です。',
  author: {
    id: 'user-1',
    pubkey: 'pubkey-1',
    npub: 'npub-1',
    name: 'フォロー中ユーザー',
    displayName: 'Followed User',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  },
  topicId: 'topic-1',
  created_at: Math.floor(Date.now() / 1000) - 120,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
  ...overrides,
});

interface FollowingQueryOverride {
  posts?: Post[];
  isLoading?: boolean;
  isError?: boolean;
  error?: unknown;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  fetchNextPage?: () => void;
  refetch?: () => void;
  serverTime?: number;
  isFetching?: boolean;
}

const mockFollowingQuery = (options: FollowingQueryOverride = {}) => {
  const {
    posts = [buildPost()],
    isLoading = false,
    isError = false,
    error = null,
    hasNextPage = false,
    isFetchingNextPage = false,
    fetchNextPage = vi.fn(),
    refetch = vi.fn(),
    serverTime = Date.now(),
    isFetching = false,
  } = options;

  const data =
    isLoading && posts.length === 0 && !isError
      ? undefined
      : {
          pages: [
            {
              cursor: null,
              items: posts,
              nextCursor: null,
              hasMore: hasNextPage,
              serverTime,
            },
          ],
          pageParams: [null],
        };

  followingMocks.useFollowingFeedQueryMock.mockReturnValue({
    data,
    isLoading,
    isError,
    error,
    refetch,
    hasNextPage,
    isFetchingNextPage,
    fetchNextPage,
    isFetching,
  });
};

function renderFollowingPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <FollowingPage />
    </QueryClientProvider>,
  );
}

describe('FollowingPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('ローディング中はスピナーを表示する', () => {
    mockFollowingQuery({ isLoading: true, posts: [] });

    renderFollowingPage();

    expect(screen.getByTestId('following-loading')).toBeInTheDocument();
  });

  it('フォロー中の投稿を表示する', () => {
    const post = buildPost({ id: 'post-123', content: '最近の投稿です。' });
    mockFollowingQuery({ posts: [post] });

    renderFollowingPage();

    expect(screen.getByText('フォロー中')).toBeInTheDocument();
    expect(screen.getByTestId('following-posts')).toBeInTheDocument();
    expect(screen.getByTestId('following-post-post-123')).toBeInTheDocument();
    expect(screen.getByText('最近の投稿です。')).toBeInTheDocument();
  });

  it('投稿が存在しない場合は空状態を表示する', () => {
    mockFollowingQuery({ posts: [] });

    renderFollowingPage();

    expect(screen.getByTestId('following-empty')).toBeInTheDocument();
    expect(screen.getByText('フォロー中の投稿はまだありません')).toBeInTheDocument();
  });

  it('エラー発生時はメッセージと再試行ボタンを表示する', async () => {
    const refetch = vi.fn();
    mockFollowingQuery({
      posts: [],
      isError: true,
      error: new Error('取得に失敗しました'),
      refetch,
    });

    const user = userEvent.setup();
    renderFollowingPage();

    expect(screen.getByTestId('following-error')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '再試行' }));
    expect(refetch).toHaveBeenCalled();
  });

  it('さらに読み込むボタンで次ページを要求する', async () => {
    const fetchNextPage = vi.fn();
    mockFollowingQuery({
      posts: [buildPost({ id: 'post-1' })],
      hasNextPage: true,
      fetchNextPage,
    });

    const user = userEvent.setup();
    renderFollowingPage();

    const loadMoreButton = screen.getByTestId('following-load-more');
    await user.click(loadMoreButton);
    expect(fetchNextPage).toHaveBeenCalled();
  });

  it('Summary Panel でフォロー中フィードの統計を表示する', () => {
    const baseAuthor = buildPost().author;
    const posts = [
      buildPost({
        id: 'post-1',
        author: { ...baseAuthor, id: 'user-1', npub: 'npub-1', pubkey: 'pubkey-1' },
      }),
      buildPost({
        id: 'post-2',
        author: { ...baseAuthor, id: 'user-2', npub: 'npub-2', pubkey: 'pubkey-2' },
      }),
    ];

    mockFollowingQuery({
      posts,
      hasNextPage: true,
      serverTime: Date.now() - 30_000,
      isFetching: false,
    });

    renderFollowingPage();

    expect(screen.getByTestId('following-summary-panel')).toBeInTheDocument();
    expect(screen.getByTestId('following-summary-posts')).toHaveTextContent('2件');
    expect(screen.getByTestId('following-summary-authors')).toHaveTextContent('2人');
    expect(screen.getByTestId('following-summary-remaining')).toHaveTextContent('あり');
    expect(screen.getByTestId('following-summary-direct-messages')).toHaveTextContent('0件');
  });
});
