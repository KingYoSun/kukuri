import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TopicThreadsPage } from '@/routes/topics.$topicId.threads';
import type { TopicTimelineEntry } from '@/hooks/usePosts';

const hooksMocks = vi.hoisted(() => ({
  useTopicThreadsMock: vi.fn(),
}));

vi.mock('@/hooks', () => ({
  useTopicThreads: hooksMocks.useTopicThreadsMock,
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
    createFileRoute: () => () => ({ useParams: () => ({ topicId: 'topic-1' }) }),
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

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TopicThreadsPage topicId="topic-1" />
    </QueryClientProvider>,
  );
}

describe('TopicThreadsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hooksMocks.useTopicThreadsMock.mockReturnValue({
      data: [buildTimelineEntry('thread-1')],
      isLoading: false,
    });
  });

  it('スレッド一覧を表示する', () => {
    renderPage();

    expect(screen.getByTestId('thread-list-title')).toBeInTheDocument();
    expect(screen.getByTestId('thread-list-items')).toBeInTheDocument();
    expect(screen.getByTestId('mock-thread-card-thread-1')).toHaveTextContent('parent-thread-1');
    expect(screen.getByTestId('thread-list-back-to-topic')).toBeInTheDocument();
  });

  it('ローディング表示を出す', () => {
    hooksMocks.useTopicThreadsMock.mockReturnValue({
      data: undefined,
      isLoading: true,
    });

    renderPage();

    expect(screen.getByTestId('thread-list-loading')).toBeInTheDocument();
  });

  it('データがない場合は空状態を表示する', () => {
    hooksMocks.useTopicThreadsMock.mockReturnValue({
      data: [],
      isLoading: false,
    });

    renderPage();

    expect(screen.getByTestId('thread-list-empty')).toBeInTheDocument();
  });
});
