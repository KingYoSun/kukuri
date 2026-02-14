import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';

import { PostSearchResults } from '@/components/search/PostSearchResults';
import { useDebounce } from '@/hooks/useDebounce';
import { usePosts } from '@/hooks/usePosts';
import { communityNodeApi } from '@/lib/api/communityNode';
import { errorHandler } from '@/lib/errorHandler';
import { type Post, useTopicStore } from '@/stores';

vi.mock('@/components/posts/PostCard', () => ({
  PostCard: ({ post }: { post: { content: string } }) => (
    <div data-testid="post-card">{post.content}</div>
  ),
}));

vi.mock('@/hooks/useDebounce', () => ({
  useDebounce: vi.fn((value: string) => value),
}));

vi.mock('@/hooks/usePosts', () => ({
  usePosts: vi.fn(),
}));

vi.mock('@/lib/api/communityNode', () => ({
  communityNodeApi: {
    getConfig: vi.fn(),
    search: vi.fn(),
  },
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

vi.mock('@/stores', () => ({
  useTopicStore: vi.fn(),
}));

const usePostsMock = usePosts as unknown as Mock;
const useDebounceMock = useDebounce as unknown as Mock;
const useTopicStoreMock = useTopicStore as unknown as Mock;
const communityNodeApiMock = communityNodeApi as unknown as {
  getConfig: Mock;
  search: Mock;
};
const errorHandlerMock = errorHandler as unknown as { log: Mock };

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

const renderWithClient = (query: string) => {
  const client = createQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <PostSearchResults query={query} />
    </QueryClientProvider>,
  );
};

const createLocalPost = (overrides: Partial<Post> = {}): Post => ({
  id: 'post-1',
  content: 'Alice のローカル投稿',
  author: {
    id: 'user-1',
    pubkey: 'pubkey-1',
    npub: 'npub-1',
    name: 'Alice',
    displayName: 'Alice',
    picture: '',
    about: '',
    nip05: '',
    publicProfile: true,
    showOnlineStatus: false,
  },
  topicId: 'topic-1',
  created_at: 1,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
  ...overrides,
});

beforeEach(() => {
  vi.clearAllMocks();

  useDebounceMock.mockImplementation((value: string) => value);
  usePostsMock.mockReturnValue({
    data: [],
    isLoading: false,
  });
  useTopicStoreMock.mockReturnValue({
    currentTopic: { id: 'topic-1', name: 'Topic One' },
    joinedTopics: ['topic-1'],
    topics: new Map([['topic-1', { id: 'topic-1', name: 'Topic One' }]]),
  });

  communityNodeApiMock.getConfig.mockResolvedValue({ nodes: [] });
  communityNodeApiMock.search.mockResolvedValue({
    topic: 'topic-1',
    query: 'alice',
    items: [],
    next_cursor: null,
    total: 0,
  });
});

describe('PostSearchResults', () => {
  it('falls back to local search when Community Node search is unavailable', async () => {
    usePostsMock.mockReturnValue({
      data: [createLocalPost()],
      isLoading: false,
    });
    communityNodeApiMock.getConfig.mockResolvedValue({
      nodes: [
        {
          base_url: 'https://community.example',
          roles: { labels: true, trust: true, search: false, bootstrap: true },
          has_token: true,
        },
      ],
    });

    renderWithClient('alice');

    expect(await screen.findByText('1件の投稿が見つかりました')).toBeInTheDocument();
    expect(screen.getByTestId('post-card')).toHaveTextContent('Alice のローカル投稿');
    expect(screen.queryByTestId('community-node-search-results')).not.toBeInTheDocument();
  });

  it('uses Community Node search when a search-enabled node with token exists', async () => {
    communityNodeApiMock.getConfig.mockResolvedValue({
      nodes: [
        {
          base_url: 'https://community.example',
          roles: { labels: true, trust: true, search: true, bootstrap: true },
          has_token: true,
        },
      ],
    });
    communityNodeApiMock.search.mockResolvedValue({
      topic: 'topic-1',
      query: 'alice',
      items: [
        {
          event_id: 'event-1',
          topic_id: 'topic-1',
          title: 'Community result',
          summary: 'summary',
          content: 'content',
          author: 'author-1',
          created_at: 1,
          tags: [],
        },
      ],
      next_cursor: null,
      total: 1,
    });

    renderWithClient('alice');

    expect(await screen.findByTestId('community-node-search-results')).toBeInTheDocument();
    expect(screen.getByTestId('community-node-search-summary')).toHaveTextContent(
      '1件の投稿が見つかりました',
    );
    expect(screen.getByText('Community result')).toBeInTheDocument();
    await waitFor(() => {
      expect(communityNodeApiMock.search).toHaveBeenCalled();
    });
  });

  it('shows error state and logs through errorHandler when Community Node search fails', async () => {
    communityNodeApiMock.getConfig.mockResolvedValue({
      nodes: [
        {
          base_url: 'https://community.example',
          roles: { labels: true, trust: true, search: true, bootstrap: true },
          has_token: true,
        },
      ],
    });
    communityNodeApiMock.search.mockRejectedValue(new Error('community node failed'));

    renderWithClient('error-case');

    expect(
      await screen.findByText('検索に失敗しました。設定や接続状況を確認してください。'),
    ).toBeInTheDocument();

    await waitFor(() => {
      expect(errorHandlerMock.log).toHaveBeenCalledWith(
        'CommunityNode.search_failed',
        expect.any(Error),
        expect.objectContaining({
          context: 'PostSearchResults.communityNode',
          metadata: { topicId: 'topic-1', query: 'error-case' },
        }),
      );
    });
  });

  it('loads next page when clicking load more in Community Node search', async () => {
    const user = userEvent.setup();
    communityNodeApiMock.getConfig.mockResolvedValue({
      nodes: [
        {
          base_url: 'https://community.example',
          roles: { labels: true, trust: true, search: true, bootstrap: true },
          has_token: true,
        },
      ],
    });
    communityNodeApiMock.search
      .mockResolvedValueOnce({
        topic: 'topic-1',
        query: 'rust',
        items: [
          {
            event_id: 'event-1',
            topic_id: 'topic-1',
            title: 'Page 1',
            summary: 'summary-1',
            content: 'content-1',
            author: 'author-1',
            created_at: 1,
            tags: [],
          },
        ],
        next_cursor: 'cursor-1',
        total: 2,
      })
      .mockResolvedValueOnce({
        topic: 'topic-1',
        query: 'rust',
        items: [
          {
            event_id: 'event-2',
            topic_id: 'topic-1',
            title: 'Page 2',
            summary: 'summary-2',
            content: 'content-2',
            author: 'author-2',
            created_at: 2,
            tags: [],
          },
        ],
        next_cursor: null,
        total: 2,
      });

    renderWithClient('rust');

    expect(await screen.findByText('Page 1')).toBeInTheDocument();

    await user.click(await screen.findByTestId('community-node-search-load-more'));

    expect(await screen.findByText('Page 2')).toBeInTheDocument();
    expect(screen.getAllByTestId('community-node-search-result')).toHaveLength(2);
    await waitFor(() => {
      expect(communityNodeApiMock.search).toHaveBeenNthCalledWith(2, {
        topic: 'topic-1',
        q: 'rust',
        limit: 5,
        cursor: 'cursor-1',
      });
    });
  });
});
