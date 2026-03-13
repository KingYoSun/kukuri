import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ForumThreadView } from '@/components/posts/ForumThreadView';
import { buildThreadTree } from '@/components/posts/forumThreadTree';
import type { Post } from '@/stores/types';

vi.mock('@/components/posts/PostCard', () => ({
  PostCard: ({ post }: { post: Post }) => (
    <article data-testid={`mock-thread-post-${post.id}`}>{post.content}</article>
  ),
}));

const buildPost = (
  id: string,
  createdAt: number,
  options?: {
    parentId?: string | null;
    rootId?: string;
  },
): Post => ({
  id,
  content: `content-${id}`,
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
  threadRootEventId: options?.rootId ?? 'root',
  threadParentEventId: options?.parentId ?? null,
  created_at: createdAt,
  tags: [],
  likes: 0,
  boosts: 0,
  replies: [],
});

describe('ForumThreadView', () => {
  it('root と階層返信を描画する', () => {
    const posts: Post[] = [
      buildPost('root', 10, { rootId: 'root' }),
      buildPost('reply-1', 20, { parentId: 'root', rootId: 'root' }),
      buildPost('reply-2', 30, { parentId: 'reply-1', rootId: 'root' }),
      buildPost('reply-3', 40, { parentId: 'root', rootId: 'root' }),
    ];

    render(<ForumThreadView threadUuid="thread-1" posts={posts} />);

    expect(screen.getByTestId('forum-thread-root')).toBeInTheDocument();
    expect(screen.getByTestId('mock-thread-post-root')).toBeInTheDocument();
    expect(screen.getByTestId('forum-thread-node-reply-1')).toBeInTheDocument();
    expect(screen.getByTestId('forum-thread-node-reply-2')).toBeInTheDocument();
    expect(screen.getByTestId('forum-thread-node-reply-3')).toBeInTheDocument();
  });

  it('expand/collapse で子返信の表示を切り替えられる', async () => {
    const user = userEvent.setup();
    const posts: Post[] = [
      buildPost('root', 10, { rootId: 'root' }),
      buildPost('reply-1', 20, { parentId: 'root', rootId: 'root' }),
      buildPost('reply-2', 30, { parentId: 'reply-1', rootId: 'root' }),
    ];

    render(<ForumThreadView threadUuid="thread-1" posts={posts} />);

    expect(screen.getByTestId('forum-thread-children-reply-1')).toBeInTheDocument();

    await user.click(screen.getByTestId('forum-thread-toggle-reply-1'));
    expect(screen.queryByTestId('forum-thread-children-reply-1')).toBeNull();

    await user.click(screen.getByTestId('forum-thread-toggle-reply-1'));
    expect(screen.getByTestId('forum-thread-children-reply-1')).toBeInTheDocument();
  });

  it('parent が存在しない投稿を detached roots に分類する', () => {
    const posts: Post[] = [
      buildPost('root', 10, { rootId: 'root' }),
      buildPost('detached', 20, { parentId: 'missing-parent', rootId: 'root' }),
    ];

    const tree = buildThreadTree(posts);

    expect(tree.root?.post.id).toBe('root');
    expect(tree.detachedRoots).toHaveLength(1);
    expect(tree.detachedRoots[0]?.post.id).toBe('detached');
  });
});
