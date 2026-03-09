import { beforeEach, describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { TopicThreadsPage, TopicThreadsRoute } from '@/routes/topics.$topicId.threads';
import type { TopicTimelineEntry } from '@/hooks/usePosts';

const hooksMocks = vi.hoisted(() => ({
  useTopicThreadsMock: vi.fn(),
}));

const routerMocks = vi.hoisted(() => ({
  pathname: '/topics/topic-1/threads',
  topicId: 'topic-1',
}));

vi.mock('@/hooks', () => ({
  useTopicThreads: hooksMocks.useTopicThreadsMock,
}));

vi.mock('@/stores', () => ({
  useTopicStore: (selector: (state: { topics: Map<string, { name: string }> }) => unknown) =>
    selector({
      topics: new Map([['topic-1', { name: 'topic-1' }]]),
    }),
}));

vi.mock('@/components/posts/TimelineThreadCard', () => ({
  TimelineThreadCard: ({ entry }: { entry: TopicTimelineEntry }) => (
    <article data-testid={`mock-thread-card-${entry.threadUuid}`}>
      {entry.parentPost.content}
    </article>
  ),
}));

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  return {
    ...actual,
    Link: ({ children, to: _to, params: _params, ...rest }: any) => <a {...rest}>{children}</a>,
    Outlet: () => <div data-testid="mock-topic-threads-outlet" />,
    createFileRoute: () => () => ({ useParams: () => ({ topicId: routerMocks.topicId }) }),
    useLocation: (options?: { select?: (location: { pathname: string }) => unknown }) => {
      const location = { pathname: routerMocks.pathname };
      return options?.select ? options.select(location) : location;
    },
  };
});

const buildTimelineEntry = (threadUuid: string): TopicTimelineEntry => ({
  threadUuid,
  parentPost: {
    id: `parent-${threadUuid}`,
    content: `parent-${threadUuid}`,
    author: {
      id: 'author-1',
      pubkey: 'pubkey-1',
      npub: 'npub-1',
      name: 'Test User',
      displayName: 'Test User',
      picture: '',
      about: '',
      nip05: '',
      avatar: null,
      publicProfile: true,
      showOnlineStatus: false,
    },
    topicId: 'topic-1',
    created_at: 1,
    tags: [],
    likes: 0,
    boosts: 0,
    replies: [],
  },
  firstReply: null,
  replyCount: 0,
  lastActivityAt: 1,
});

function renderWithQueryClient(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
}

describe('TopicThreads route', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    routerMocks.topicId = 'topic-1';
    routerMocks.pathname = '/topics/topic-1/threads';
    hooksMocks.useTopicThreadsMock.mockReturnValue({
      data: [buildTimelineEntry('thread-1')],
      isLoading: false,
    });
  });

  it('スレッド一覧ページを表示する', () => {
    renderWithQueryClient(<TopicThreadsPage topicId="topic-1" />);

    expect(screen.getByTestId('thread-list-title')).toBeInTheDocument();
    expect(screen.getByTestId('thread-list-items')).toBeInTheDocument();
    expect(screen.getByTestId('mock-thread-card-thread-1')).toHaveTextContent('parent-thread-1');
    expect(screen.getByTestId('thread-list-back-to-topic')).toBeInTheDocument();
  });

  it('ローディング状態を表示する', () => {
    hooksMocks.useTopicThreadsMock.mockReturnValue({
      data: undefined,
      isLoading: true,
    });

    renderWithQueryClient(<TopicThreadsPage topicId="topic-1" />);

    expect(screen.getByTestId('thread-list-loading')).toBeInTheDocument();
  });

  it('データがない場合は空状態を表示する', () => {
    hooksMocks.useTopicThreadsMock.mockReturnValue({
      data: [],
      isLoading: false,
    });

    renderWithQueryClient(<TopicThreadsPage topicId="topic-1" />);

    expect(screen.getByTestId('thread-list-empty')).toBeInTheDocument();
  });

  it('スレッド詳細ルートでは Outlet を表示する', () => {
    routerMocks.pathname = '/topics/topic-1/threads/thread-1';

    renderWithQueryClient(<TopicThreadsRoute />);

    expect(screen.getByTestId('mock-topic-threads-outlet')).toBeInTheDocument();
  });

  it('エンコード済み topicId の詳細ルートでも Outlet を表示する', () => {
    routerMocks.topicId = 'kukuri:tauri:thread-route';
    routerMocks.pathname = '/topics/kukuri%3Atauri%3Athread-route/threads/thread-1';

    renderWithQueryClient(<TopicThreadsRoute />);

    expect(screen.getByTestId('mock-topic-threads-outlet')).toBeInTheDocument();
  });
});
