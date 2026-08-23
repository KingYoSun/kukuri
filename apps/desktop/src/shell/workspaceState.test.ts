import { describe, expect, it } from 'vitest';

import {
  activateColumn,
  closeColumn,
  columnSpanPolicy,
  columnIdentityId,
  createInitialWorkspaceState,
  defaultColumnSpan,
  INITIAL_TIMELINE_COLUMN_ID,
  moveColumn,
  openTransientColumn,
  openPinnedColumn,
  setColumnSpan,
  setColumnPinned,
  type ColumnState,
} from '@/shell/slices/workspace';

function transientColumn(overrides: Partial<ColumnState> = {}): ColumnState {
  return {
    id: 'thread-1',
    kind: 'thread',
    entityId: 'thread-1',
    parentColumnId: INITIAL_TIMELINE_COLUMN_ID,
    pinned: false,
    preferredDesktopSpan: 1,
    ...overrides,
  };
}

describe('workspace state transitions', () => {
  it.each([
    ['timeline', 1, 1, 1],
    ['notifications', 1, 1, 1],
    ['thread', 1, 1, 1],
    ['profile', 1, 1, 1],
    ['explore', 1, 1, 1],
    ['messages', 1, 1, 2],
    ['conversation', 1, 1, 2],
    ['stream', 2, 1, 2],
    ['game', 1, 1, 1],
    ['metaverse', 3, 1, 4],
  ] as const)('defines %s span policy', (kind, defaultSpan, min, max) => {
    expect(columnSpanPolicy(kind)).toEqual({ default: defaultSpan, min, max });
    expect(defaultColumnSpan(kind)).toBe(defaultSpan);
  });

  it('starts with one active Timeline Column', () => {
    const state = createInitialWorkspaceState({
      topicId: 'kukuri:topic:demo',
      channelId: null,
    });

    expect(state.columns).toEqual([
      {
        id: INITIAL_TIMELINE_COLUMN_ID,
        kind: 'timeline',
        scope: { topicId: 'kukuri:topic:demo', channelId: null },
        pinned: true,
        preferredDesktopSpan: 1,
      },
    ]);
    expect(state.activeColumnId).toBe(INITIAL_TIMELINE_COLUMN_ID);
  });

  it('builds stable identity from kind, scope, and entity', () => {
    expect(
      columnIdentityId('timeline', {
        topicId: 'kukuri:topic:demo',
        channelId: null,
      })
    ).toBe('column:timeline:kukuri%3Atopic%3Ademo:-:-');
    expect(
      columnIdentityId(
        'conversation',
        { topicId: 'kukuri:topic:demo', channelId: 'friends/plus' },
        'peer:alice'
      )
    ).toBe(
      'column:conversation:kukuri%3Atopic%3Ademo:friends%2Fplus:peer%3Aalice'
    );
    expect(
      columnIdentityId('timeline', {
        topicId: 'kukuri:topic:demo',
        channelId: 'friends',
      })
    ).not.toBe(
      columnIdentityId('timeline', {
        topicId: 'kukuri:topic:demo',
        channelId: null,
      })
    );
  });

  it('activates an existing Column without changing the layout', () => {
    const initial = createInitialWorkspaceState();
    const withThread = openTransientColumn(initial, transientColumn());
    const next = activateColumn(withThread, INITIAL_TIMELINE_COLUMN_ID);

    expect(next.activeColumnId).toBe(INITIAL_TIMELINE_COLUMN_ID);
    expect(next.columns).toEqual(withThread.columns);
  });

  it('replaces only an unpinned transient Column opened from the same parent', () => {
    const initial = createInitialWorkspaceState();
    const withThread = openTransientColumn(initial, transientColumn());
    const withProfile = openTransientColumn(
      withThread,
      transientColumn({ id: 'profile-1', kind: 'profile', entityId: 'author-1' })
    );

    expect(withProfile.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'profile-1',
    ]);
    expect(withProfile.activeColumnId).toBe('profile-1');
  });

  it('keeps pinned and unrelated Columns when another transient Column opens', () => {
    const initial = createInitialWorkspaceState();
    const withThread = openTransientColumn(initial, transientColumn());
    const pinnedThread = setColumnPinned(withThread, 'thread-1', true);
    const withProfile = openTransientColumn(
      pinnedThread,
      transientColumn({ id: 'profile-1', kind: 'profile', entityId: 'author-1' })
    );
    const withConversation = openTransientColumn(
      withProfile,
      transientColumn({
        id: 'conversation-1',
        kind: 'conversation',
        entityId: 'peer-1',
        parentColumnId: 'thread-1',
      })
    );

    expect(withConversation.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'profile-1',
      'thread-1',
      'conversation-1',
    ]);
  });

  it('adds an explicit pinned Column without replacing transient siblings', () => {
    const initial = openTransientColumn(createInitialWorkspaceState(), transientColumn());
    const explore = transientColumn({
      id: 'explore-demo',
      kind: 'explore',
      entityId: undefined,
      parentColumnId: undefined,
    });
    const next = openPinnedColumn(initial, explore);

    expect(next.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'thread-1',
      'explore-demo',
    ]);
    expect(next.columns.at(-1)?.pinned).toBe(true);
    expect(next.activeColumnId).toBe('explore-demo');
  });

  it('applies the kind default when a new Column omits its preferred span', () => {
    const initial = createInitialWorkspaceState();
    const next = openPinnedColumn(initial, {
      id: 'stream-demo',
      kind: 'stream',
      pinned: true,
    });

    expect(next.columns.at(-1)?.preferredDesktopSpan).toBe(2);
  });

  it('clamps span changes without touching Column identity or relationships', () => {
    const initial = createInitialWorkspaceState();
    const withMetaverse = openPinnedColumn(initial, {
      id: 'metaverse-demo',
      kind: 'metaverse',
      parentColumnId: INITIAL_TIMELINE_COLUMN_ID,
      pinned: true,
    });
    const expanded = setColumnSpan(withMetaverse, 'metaverse-demo', 4);
    const clampedTimeline = setColumnSpan(expanded, INITIAL_TIMELINE_COLUMN_ID, 4);

    expect(expanded.columns.at(-1)).toMatchObject({
      id: 'metaverse-demo',
      parentColumnId: INITIAL_TIMELINE_COLUMN_ID,
      preferredDesktopSpan: 4,
    });
    expect(clampedTimeline.columns[0].preferredDesktopSpan).toBe(1);
    expect(setColumnSpan(clampedTimeline, 'missing', 2)).toBe(clampedTimeline);
  });

  it('moves a multi-span Column atomically and preserves product state', () => {
    let state = createInitialWorkspaceState();
    state = openPinnedColumn(state, {
      id: 'stream-demo',
      kind: 'stream',
      scope: { topicId: 'kukuri:topic:stream', channelId: null },
      entityId: 'session-1',
      parentColumnId: INITIAL_TIMELINE_COLUMN_ID,
      pinned: true,
    });
    state = openPinnedColumn(state, {
      id: 'profile-demo',
      kind: 'profile',
      entityId: 'alice',
      pinned: true,
    });
    const streamBefore = state.columns[1];
    const moved = moveColumn(state, 'stream-demo', 0);

    expect(moved.columns.map((column) => column.id)).toEqual([
      'stream-demo',
      INITIAL_TIMELINE_COLUMN_ID,
      'profile-demo',
    ]);
    expect(moved.columns[0]).toEqual(streamBefore);
    expect(moved.activeColumnId).toBe('profile-demo');
    expect(moveColumn(moved, 'stream-demo', 0)).toBe(moved);
    expect(moveColumn(moved, 'missing', 1)).toBe(moved);
  });

  it('inserts a new child immediately to the right of its parent', () => {
    const initial = createInitialWorkspaceState();
    const withProfile = openTransientColumn(
      initial,
      transientColumn({ id: 'profile-1', kind: 'profile', parentColumnId: undefined })
    );
    const withThread = openTransientColumn(
      withProfile,
      transientColumn({ parentColumnId: INITIAL_TIMELINE_COLUMN_ID })
    );

    expect(withThread.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'thread-1',
      'profile-1',
    ]);
  });

  it('reparents and reorders an existing transient Column when its causal chain changes', () => {
    const profile = transientColumn({
      id: 'profile-bob',
      kind: 'profile',
      entityId: 'bob',
    });
    let state = openTransientColumn(createInitialWorkspaceState(), profile);
    state = openTransientColumn(
      state,
      transientColumn({ id: 'messages', kind: 'messages', parentColumnId: undefined })
    );
    state = openTransientColumn(
      state,
      transientColumn({
        id: 'conversation-bob',
        kind: 'conversation',
        entityId: 'bob',
        parentColumnId: 'messages',
      })
    );
    state = openTransientColumn(state, {
      ...profile,
      parentColumnId: 'conversation-bob',
    });

    expect(state.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'messages',
      'conversation-bob',
      'profile-bob',
    ]);
    expect(state.columns.at(-1)?.parentColumnId).toBe('conversation-bob');
  });

  it('does not let a re-opened transient Column adopt itself as its parent', () => {
    const profile = transientColumn({
      id: 'profile-alice',
      kind: 'profile',
      entityId: 'alice',
    });
    const withProfile = openTransientColumn(createInitialWorkspaceState(), profile);
    // 自分自身を parent に指定して再 open しても、自己参照にはならず既存の parent を維持する。
    const reopened = openTransientColumn(withProfile, {
      ...profile,
      parentColumnId: 'profile-alice',
    });

    expect(reopened.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'profile-alice',
    ]);
    expect(reopened.columns.at(-1)?.parentColumnId).toBe(INITIAL_TIMELINE_COLUMN_ID);

    // 以後、同じ parent から別 author を開くと置き換わる(蓄積しない)。
    const replaced = openTransientColumn(
      reopened,
      transientColumn({ id: 'profile-bob', kind: 'profile', entityId: 'bob' })
    );
    expect(replaced.columns.map((column) => column.id)).toEqual([
      INITIAL_TIMELINE_COLUMN_ID,
      'profile-bob',
    ]);
  });

  it('drops a self-referencing parent when a new transient Column is opened', () => {
    const next = openTransientColumn(
      createInitialWorkspaceState(),
      transientColumn({ id: 'profile-alice', kind: 'profile', parentColumnId: 'profile-alice' })
    );

    expect(next.columns.at(-1)?.parentColumnId).toBeUndefined();
  });

  it('moves active state to the parent, then a neighbor, when closing a Column', () => {
    const initial = createInitialWorkspaceState();
    const withThread = openTransientColumn(initial, transientColumn());
    const afterThreadClose = closeColumn(withThread, 'thread-1');

    expect(afterThreadClose.activeColumnId).toBe(INITIAL_TIMELINE_COLUMN_ID);
    expect(afterThreadClose.columns).toHaveLength(1);

    const withOrphan = openTransientColumn(
      initial,
      transientColumn({ id: 'profile-1', kind: 'profile', parentColumnId: undefined })
    );
    const afterTimelineClose = closeColumn(withOrphan, INITIAL_TIMELINE_COLUMN_ID);

    expect(afterTimelineClose.activeColumnId).toBe('profile-1');
    expect(afterTimelineClose.columns).toHaveLength(1);
  });
});
