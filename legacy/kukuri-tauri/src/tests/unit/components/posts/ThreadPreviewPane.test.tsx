import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThreadPreviewPane } from '@/components/posts/ThreadPreviewPane';
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

const buildPost = (id: string): Post => ({
  id,
  content: id,
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
  threadUuid: 'thread-1',
  threadRootEventId: 'root-1',
  threadParentEventId: null,
  created_at: 1,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
});

describe('ThreadPreviewPane', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hooksMocks.useThreadPostsMock.mockReturnValue({
      data: [buildPost('root-1'), buildPost('reply-1')],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    });
  });

  it('アクセシビリティ代替ボタンで全画面遷移コールバックを呼ぶ', async () => {
    const user = userEvent.setup();
    const onOpenFullThread = vi.fn();

    render(
      <ThreadPreviewPane
        topicId="topic-1"
        threadUuid="thread-1"
        onClose={vi.fn()}
        onOpenFullThread={onOpenFullThread}
      />,
    );

    await user.click(screen.getByTestId('thread-preview-open-full'));
    expect(onOpenFullThread).toHaveBeenCalledTimes(1);
  });

  it('左ドラッグ閾値を超えると全画面遷移コールバックを呼ぶ', () => {
    const onOpenFullThread = vi.fn();

    render(
      <ThreadPreviewPane
        topicId="topic-1"
        threadUuid="thread-1"
        onClose={vi.fn()}
        onOpenFullThread={onOpenFullThread}
      />,
    );

    const pane = screen.getByTestId('thread-preview-pane');
    fireEvent.pointerDown(pane, { pointerId: 1, clientX: 240, pointerType: 'mouse', button: 0 });
    fireEvent.pointerMove(pane, { pointerId: 1, clientX: 100, pointerType: 'mouse' });
    fireEvent.pointerUp(pane, { pointerId: 1, clientX: 100, pointerType: 'mouse' });

    expect(onOpenFullThread).toHaveBeenCalledTimes(1);
  });

  it('左ドラッグが閾値未満なら全画面遷移しない', () => {
    const onOpenFullThread = vi.fn();

    render(
      <ThreadPreviewPane
        topicId="topic-1"
        threadUuid="thread-1"
        onClose={vi.fn()}
        onOpenFullThread={onOpenFullThread}
      />,
    );

    const pane = screen.getByTestId('thread-preview-pane');
    fireEvent.pointerDown(pane, { pointerId: 1, clientX: 240, pointerType: 'mouse', button: 0 });
    fireEvent.pointerMove(pane, { pointerId: 1, clientX: 150, pointerType: 'mouse' });
    fireEvent.pointerUp(pane, { pointerId: 1, clientX: 150, pointerType: 'mouse' });

    expect(onOpenFullThread).not.toHaveBeenCalled();
  });
});
