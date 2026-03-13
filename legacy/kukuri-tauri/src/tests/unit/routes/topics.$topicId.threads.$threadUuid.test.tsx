import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TopicThreadDetailPage } from '@/routes/topics.$topicId.threads.$threadUuid';
import type { Post } from '@/stores/types';

const hooksMocks = vi.hoisted(() => ({
  useThreadPostsMock: vi.fn(),
}));

vi.mock('@/hooks', () => ({
  useThreadPosts: hooksMocks.useThreadPostsMock,
}));

vi.mock('@/components/posts/ForumThreadView', () => ({
  ForumThreadView: ({ threadUuid, posts }: { threadUuid: string; posts: Post[] }) => (
    <section data-testid={`mock-forum-thread-${threadUuid}`}>{posts.length}</section>
  ),
}));

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  return {
    ...actual,
    Link: ({ children, to: _to, params: _params, ...rest }: any) => <a {...rest}>{children}</a>,
    createFileRoute: () => () => ({
      useParams: () => ({ topicId: 'topic-1', threadUuid: 'thread-1' }),
    }),
  };
});

const buildPost = (id: string): Post => ({
  id,
  content: id,
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
  threadUuid: 'thread-1',
  threadRootEventId: 'root-1',
  threadParentEventId: null,
  created_at: 1,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
});

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TopicThreadDetailPage topicId="topic-1" threadUuid="thread-1" />
    </QueryClientProvider>,
  );
}

describe('TopicThreadDetailPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hooksMocks.useThreadPostsMock.mockReturnValue({
      data: [buildPost('root-1'), buildPost('reply-1')],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

  it('thread detail を表示する', () => {
    renderPage();

    expect(screen.getByTestId('thread-detail-title')).toBeInTheDocument();
    expect(screen.getByTestId('mock-forum-thread-thread-1')).toHaveTextContent('2');
    expect(screen.getByTestId('thread-detail-back-to-list')).toBeInTheDocument();
    expect(screen.getByTestId('thread-detail-back-to-topic')).toBeInTheDocument();
  });

  it('ローディング表示を出す', () => {
    hooksMocks.useThreadPostsMock.mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
      refetch: vi.fn(),
    });

    renderPage();

    expect(screen.getByTestId('thread-detail-loading')).toBeInTheDocument();
  });

  it('取得エラー時に再試行できる', async () => {
    const user = userEvent.setup();
    const refetchMock = vi.fn();
    hooksMocks.useThreadPostsMock.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('failed'),
      refetch: refetchMock,
    });

    renderPage();

    expect(screen.getByTestId('thread-detail-error')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '再試行' }));
    expect(refetchMock).toHaveBeenCalledTimes(1);
  });

  it('投稿が空の場合は未検出メッセージを表示する', () => {
    hooksMocks.useThreadPostsMock.mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });

    renderPage();

    expect(screen.getByTestId('thread-detail-empty')).toBeInTheDocument();
  });
});
