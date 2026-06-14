import { expect, test } from 'vitest';

import { buildThreadTree } from './buildThreadTree';
import { type PostCardView } from './types';

function node(
  objectId: string,
  options?: {
    replyTo?: string | null;
    rootId?: string | null;
    createdAt?: number;
    serverObjectId?: string | null;
    localId?: string | null;
  }
): PostCardView {
  return {
    post: {
      object_id: objectId,
      envelope_id: `envelope-${objectId}`,
      author_pubkey: 'a'.repeat(64),
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: false,
      object_kind: 'post',
      content: objectId,
      content_status: 'Available',
      attachments: [],
      created_at: options?.createdAt ?? 0,
      reply_to: options?.replyTo ?? null,
      root_id: options?.rootId ?? objectId,
      repost_of: null,
      repost_commentary: null,
      audience_label: 'Public',
      reaction_summary: [],
      my_reactions: [],
      server_object_id: options?.serverObjectId ?? null,
      local_id: options?.localId ?? null,
    },
    context: 'thread',
    authorLabel: objectId,
    relationshipLabel: null,
    threadTargetId: 'root',
    media: {
      objectId,
      kind: null,
      extraAttachmentCount: 0,
      state: 'ready',
      videoUnsupportedOnClient: false,
    },
  };
}

function shape(posts: PostCardView[]) {
  return buildThreadTree(posts).map((entry) => ({
    id: entry.view.post.object_id,
    depth: entry.depth,
  }));
}

test('linear reply chain nests by depth', () => {
  const posts = [
    node('root', { createdAt: 0 }),
    node('a', { replyTo: 'root', rootId: 'root', createdAt: 1 }),
    node('b', { replyTo: 'a', rootId: 'root', createdAt: 2 }),
  ];

  expect(shape(posts)).toEqual([
    { id: 'root', depth: 0 },
    { id: 'a', depth: 1 },
    { id: 'b', depth: 2 },
  ]);
});

test('branches order siblings chronologically (pre-order DFS)', () => {
  const posts = [
    node('root', { createdAt: 0 }),
    node('late', { replyTo: 'root', rootId: 'root', createdAt: 5 }),
    node('early', { replyTo: 'root', rootId: 'root', createdAt: 2 }),
    node('early-child', { replyTo: 'early', rootId: 'root', createdAt: 3 }),
  ];

  expect(shape(posts)).toEqual([
    { id: 'root', depth: 0 },
    { id: 'early', depth: 1 },
    { id: 'early-child', depth: 2 },
    { id: 'late', depth: 1 },
  ]);
});

test('orphan reply (parent not loaded) is promoted to a root', () => {
  const posts = [
    node('root', { createdAt: 0 }),
    node('orphan', { replyTo: 'missing-parent', rootId: 'root', createdAt: 1 }),
  ];

  expect(shape(posts)).toEqual([
    { id: 'root', depth: 0 },
    { id: 'orphan', depth: 0 },
  ]);
});

test('reply_to resolves against server_object_id and local_id', () => {
  const posts = [
    node('root', { createdAt: 0, serverObjectId: 'server-root' }),
    node('child', { replyTo: 'server-root', rootId: 'root', createdAt: 1 }),
  ];

  expect(shape(posts)).toEqual([
    { id: 'root', depth: 0 },
    { id: 'child', depth: 1 },
  ]);
});

test('a reply cycle never loops and every post is emitted once', () => {
  const posts = [
    node('x', { replyTo: 'y', rootId: 'x', createdAt: 0 }),
    node('y', { replyTo: 'x', rootId: 'x', createdAt: 1 }),
  ];

  const result = shape(posts);
  expect(result).toHaveLength(2);
  expect(new Set(result.map((entry) => entry.id))).toEqual(new Set(['x', 'y']));
});

test('a self-reply is treated as a root', () => {
  const posts = [node('self', { replyTo: 'self', rootId: 'self', createdAt: 0 })];
  expect(shape(posts)).toEqual([{ id: 'self', depth: 0 }]);
});
