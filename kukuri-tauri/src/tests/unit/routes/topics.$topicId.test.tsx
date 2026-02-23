import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TopicPage } from '@/routes/topics.$topicId';
import type { TopicTimelineEntry } from '@/hooks/usePosts';
import type { Topic } from '@/stores';

const hooksMocks = vi.hoisted(() => ({
  useTopicTimelineMock: vi.fn(),
  useRealtimeTimelineMock: vi.fn(),
}));

const storeMocks = vi.hoisted(() => ({
  useTopicStoreMock: vi.fn(),
}));

const routerMocks = vi.hoisted(() => ({
  navigateMock: vi.fn(),
  pathname: '/topics/topic-1',
}));

const uiStoreMocks = vi.hoisted(() => {
  const state = {
    timelineUpdateMode: 'standard' as const,
    setTimelineUpdateMode: vi.fn(),
  };
  return {
    state,
    useUIStoreMock: vi.fn((selector: (value: typeof state) => unknown) => selector(state)),
  };
});

vi.mock('@/hooks', () => ({
  useTopicTimeline: hooksMocks.useTopicTimelineMock,
  useRealtimeTimeline: hooksMocks.useRealtimeTimelineMock,
}));

vi.mock('@/stores', () => ({
  useTopicStore: storeMocks.useTopicStoreMock,
}));

vi.mock('@/stores/uiStore', () => ({
  useUIStore: uiStoreMocks.useUIStoreMock,
}));

vi.mock('@/components/posts/TimelineThreadCard', () => ({
  TimelineThreadCard: ({
    entry,
    onParentPostClick,
  }: {
    entry: TopicTimelineEntry;
    onParentPostClick?: (threadUuid: string) => void;
  }) => (
    <button
      type="button"
      data-testid={`mock-thread-parent-${entry.threadUuid}`}
      onClick={() => onParentPostClick?.(entry.threadUuid)}
    >
      {entry.parentPost.content}
    </button>
  ),
}));

vi.mock('@/components/posts/ThreadPreviewPane', () => ({
  ThreadPreviewPane: ({
    threadUuid,
    onOpenFullThread,
  }: {
    threadUuid: string;
    onOpenFullThread: () => void;
  }) => (
    <section data-testid="mock-thread-preview-pane">
      <p data-testid="mock-thread-preview-uuid">{threadUuid}</p>
      <button type="button" data-testid="mock-thread-preview-open-full" onClick={onOpenFullThread}>
        open full
      </button>
    </section>
  ),
}));

vi.mock('@/components/TopicMeshVisualization', () => ({
  TopicMeshVisualization: () => <div data-testid="mock-topic-mesh" />,
}));

vi.mock('@/components/posts/PostComposer', () => ({
  PostComposer: () => <div data-testid="mock-post-composer" />,
}));

vi.mock('@/components/topics/TopicFormModal', () => ({
  TopicFormModal: () => null,
}));

vi.mock('@/components/topics/TopicDeleteDialog', () => ({
  TopicDeleteDialog: () => null,
}));

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  return {
    ...actual,
    Outlet: () => <div data-testid="mock-topic-outlet" />,
    createFileRoute: () => () => ({
      useParams: () => ({ topicId: 'topic-1' }),
    }),
    useNavigate: () => routerMocks.navigateMock,
    useLocation: (options?: { select?: (location: { pathname: string }) => unknown }) => {
      const location = { pathname: routerMocks.pathname };
      return options?.select ? options.select(location) : location;
    },
  };
});

const buildTopic = (): Topic => ({
  id: 'topic-1',
  name: 'topic-1',
  description: 'description',
  tags: [],
  memberCount: 1,
  postCount: 1,
  lastActive: 1_700_000_000,
  isActive: true,
  createdAt: new Date(),
  visibility: 'public',
  isJoined: true,
});

const buildEntry = (): TopicTimelineEntry => ({
  threadUuid: 'thread-1',
  parentPost: {
    id: 'parent-1',
    content: 'parent-content',
    author: {
      id: 'author-1',
      pubkey: 'pubkey-1',
      npub: 'npub-1',
      name: 'Tester',
      displayName: 'Tester',
      picture: '',
      about: '',
      nip05: '',
      avatar: null,
      publicProfile: true,
      showOnlineStatus: false,
    },
    topicId: 'topic-1',
    created_at: 1_700_000_000,
    tags: [],
    likes: 0,
    boosts: 0,
    replies: [],
  },
  firstReply: null,
  replyCount: 0,
  lastActivityAt: 1_700_000_100,
});

describe('TopicPage right pane preview', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    routerMocks.pathname = '/topics/topic-1';
    uiStoreMocks.state.timelineUpdateMode = 'standard';
    hooksMocks.useTopicTimelineMock.mockReturnValue({
      data: [buildEntry()],
      isLoading: false,
      refetch: vi.fn(),
    });
    hooksMocks.useRealtimeTimelineMock.mockReturnValue(undefined);
    storeMocks.useTopicStoreMock.mockReturnValue({
      topics: new Map([['topic-1', buildTopic()]]),
      joinedTopics: ['topic-1'],
      currentTopic: null,
      pendingTopics: new Map(),
    });
  });

  it('タイムライン親投稿クリックで右ペイン preview を開く', async () => {
    const user = userEvent.setup();
    render(<TopicPage />);

    expect(screen.queryByTestId('mock-thread-preview-pane')).toBeNull();
    await user.click(screen.getByTestId('mock-thread-parent-thread-1'));

    expect(screen.getByTestId('mock-thread-preview-pane')).toBeInTheDocument();
    expect(screen.getByTestId('mock-thread-preview-uuid')).toHaveTextContent('thread-1');
  });

  it('preview の全画面ボタンで thread ルートへ遷移する', async () => {
    const user = userEvent.setup();
    render(<TopicPage />);

    await user.click(screen.getByTestId('mock-thread-parent-thread-1'));
    await user.click(screen.getByTestId('mock-thread-preview-open-full'));

    expect(routerMocks.navigateMock).toHaveBeenCalledWith({
      to: '/topics/$topicId/threads/$threadUuid',
      params: { topicId: 'topic-1', threadUuid: 'thread-1' },
    });
  });

  it('timeline mode toggle で realtime 選択時に store setter が呼ばれる', async () => {
    const user = userEvent.setup();
    render(<TopicPage />);

    await user.click(screen.getByTestId('timeline-mode-toggle-realtime'));

    expect(uiStoreMocks.state.setTimelineUpdateMode).toHaveBeenCalledWith('realtime');
  });
});
