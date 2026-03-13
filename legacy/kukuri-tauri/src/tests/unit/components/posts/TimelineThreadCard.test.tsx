import { describe, it, expect, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { TimelineThreadCard } from '@/components/posts/TimelineThreadCard';
import type { Post } from '@/stores/types';
import type { TopicTimelineEntry } from '@/hooks/usePosts';

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  return {
    ...actual,
    Link: ({ children, to: _to, params: _params, ...rest }: any) => <a {...rest}>{children}</a>,
  };
});

vi.mock('@/components/posts/PostCard', () => ({
  PostCard: ({ post }: { post: Post }) => (
    <article data-testid={`mock-post-card-${post.id}`}>
      <span>{post.content}</span>
      <button type="button" data-testid={`mock-post-card-action-${post.id}`}>
        Action
      </button>
    </article>
  ),
}));

const buildPost = (id: string, content: string): Post => ({
  id,
  content,
  author: {
    id: `author-${id}`,
    pubkey: `pubkey-${id}`,
    npub: `npub-${id}`,
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
  created_at: 1_700_000_000,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
});

describe('TimelineThreadCard', () => {
  it('親投稿と先頭返信プレビュー、件数/最終アクティビティを表示する', () => {
    const entry: TopicTimelineEntry = {
      threadUuid: 'thread-1',
      parentPost: buildPost('parent-1', 'Parent content'),
      firstReply: buildPost('reply-1', 'First reply content'),
      replyCount: 3,
      lastActivityAt: 1_700_000_500,
    };

    render(<TimelineThreadCard entry={entry} topicId="topic-1" />);

    expect(screen.getByTestId('timeline-thread-card-thread-1')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-thread-parent-thread-1')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-thread-first-reply-thread-1')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-thread-replies-thread-1')).toHaveTextContent('3');
    expect(screen.getByTestId('timeline-thread-last-activity-thread-1')).toBeInTheDocument();
    expect(screen.getByTestId('mock-post-card-parent-1')).toHaveTextContent('Parent content');
    expect(screen.getByTestId('mock-post-card-reply-1')).toHaveTextContent('First reply content');
    expect(screen.getByTestId('timeline-thread-open-thread-1')).toBeInTheDocument();
  });

  it('返信がない場合は先頭返信セクションを表示しない', () => {
    const entry: TopicTimelineEntry = {
      threadUuid: 'thread-2',
      parentPost: buildPost('parent-2', 'Only parent content'),
      firstReply: null,
      replyCount: 0,
      lastActivityAt: 1_700_001_000,
    };

    render(<TimelineThreadCard entry={entry} />);

    expect(screen.getByTestId('timeline-thread-card-thread-2')).toBeInTheDocument();
    expect(screen.queryByTestId('timeline-thread-first-reply-thread-2')).toBeNull();
    expect(screen.getByTestId('mock-post-card-parent-2')).toHaveTextContent('Only parent content');
  });

  it('親投稿クリックで preview コールバックを呼び出す', () => {
    const onParentPostClick = vi.fn();
    const entry: TopicTimelineEntry = {
      threadUuid: 'thread-3',
      parentPost: buildPost('parent-3', 'Parent for preview'),
      firstReply: null,
      replyCount: 1,
      lastActivityAt: 1_700_001_500,
    };

    render(
      <TimelineThreadCard entry={entry} topicId="topic-1" onParentPostClick={onParentPostClick} />,
    );

    fireEvent.click(screen.getByTestId('timeline-thread-parent-thread-3'));
    expect(onParentPostClick).toHaveBeenCalledWith('thread-3');
  });

  it('親投稿コンテナで Enter/Space を押すと preview コールバックを呼び出す', () => {
    const onParentPostClick = vi.fn();
    const entry: TopicTimelineEntry = {
      threadUuid: 'thread-4',
      parentPost: buildPost('parent-4', 'Parent for keyboard preview'),
      firstReply: null,
      replyCount: 1,
      lastActivityAt: 1_700_001_600,
    };

    render(<TimelineThreadCard entry={entry} onParentPostClick={onParentPostClick} />);

    const parentContainer = screen.getByTestId('timeline-thread-parent-thread-4');
    fireEvent.keyDown(parentContainer, { key: 'Enter' });
    fireEvent.keyDown(parentContainer, { key: ' ' });

    expect(onParentPostClick).toHaveBeenCalledTimes(2);
    expect(onParentPostClick).toHaveBeenNthCalledWith(1, 'thread-4');
    expect(onParentPostClick).toHaveBeenNthCalledWith(2, 'thread-4');
  });

  it('親投稿内のインタラクティブ要素で Enter/Space を押しても preview を開かない', () => {
    const onParentPostClick = vi.fn();
    const entry: TopicTimelineEntry = {
      threadUuid: 'thread-5',
      parentPost: buildPost('parent-5', 'Parent with action'),
      firstReply: null,
      replyCount: 1,
      lastActivityAt: 1_700_001_700,
    };

    render(<TimelineThreadCard entry={entry} onParentPostClick={onParentPostClick} />);

    const actionButton = screen.getByTestId('mock-post-card-action-parent-5');
    fireEvent.keyDown(actionButton, { key: 'Enter' });
    fireEvent.keyDown(actionButton, { key: ' ' });

    expect(onParentPostClick).not.toHaveBeenCalled();
  });
});
