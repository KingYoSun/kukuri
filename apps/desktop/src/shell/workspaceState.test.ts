import { describe, expect, it } from 'vitest';

import {
  activateColumn,
  closeColumn,
  columnIdentityId,
  createInitialWorkspaceState,
  openTransientColumn,
  setColumnPinned,
  type ColumnState,
} from '@/shell/slices/workspace';

function transientColumn(overrides: Partial<ColumnState> = {}): ColumnState {
  return {
    id: 'thread-1',
    kind: 'thread',
    entityId: 'thread-1',
    parentColumnId: 'timeline-initial',
    pinned: false,
    preferredDesktopSpan: 1,
    ...overrides,
  };
}

describe('workspace state transitions', () => {
  it('starts with one active Timeline Column', () => {
    const state = createInitialWorkspaceState({
      topicId: 'kukuri:topic:demo',
      channelId: null,
    });

    expect(state.columns).toEqual([
      {
        id: 'timeline-initial',
        kind: 'timeline',
        scope: { topicId: 'kukuri:topic:demo', channelId: null },
        pinned: true,
        preferredDesktopSpan: 1,
      },
    ]);
    expect(state.activeColumnId).toBe('timeline-initial');
  });

  it('builds stable identity from kind, scope, and entity', () => {
    expect(
      columnIdentityId(
        'conversation',
        { topicId: 'kukuri:topic:demo', channelId: 'friends/plus' },
        'peer:alice'
      )
    ).toBe(
      'column:conversation:kukuri%3Atopic%3Ademo:friends%2Fplus:peer%3Aalice'
    );
    expect(columnIdentityId('timeline')).toBe('timeline-initial');
  });

  it('activates an existing Column without changing the layout', () => {
    const initial = createInitialWorkspaceState();
    const withThread = openTransientColumn(initial, transientColumn());
    const next = activateColumn(withThread, 'timeline-initial');

    expect(next.activeColumnId).toBe('timeline-initial');
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
      'timeline-initial',
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
      'timeline-initial',
      'profile-1',
      'thread-1',
      'conversation-1',
    ]);
  });

  it('inserts a new child immediately to the right of its parent', () => {
    const initial = createInitialWorkspaceState();
    const withProfile = openTransientColumn(
      initial,
      transientColumn({ id: 'profile-1', kind: 'profile', parentColumnId: undefined })
    );
    const withThread = openTransientColumn(
      withProfile,
      transientColumn({ parentColumnId: 'timeline-initial' })
    );

    expect(withThread.columns.map((column) => column.id)).toEqual([
      'timeline-initial',
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
      'timeline-initial',
      'messages',
      'conversation-bob',
      'profile-bob',
    ]);
    expect(state.columns.at(-1)?.parentColumnId).toBe('conversation-bob');
  });

  it('moves active state to the parent, then a neighbor, when closing a Column', () => {
    const initial = createInitialWorkspaceState();
    const withThread = openTransientColumn(initial, transientColumn());
    const afterThreadClose = closeColumn(withThread, 'thread-1');

    expect(afterThreadClose.activeColumnId).toBe('timeline-initial');
    expect(afterThreadClose.columns).toHaveLength(1);

    const withOrphan = openTransientColumn(
      initial,
      transientColumn({ id: 'profile-1', kind: 'profile', parentColumnId: undefined })
    );
    const afterTimelineClose = closeColumn(withOrphan, 'timeline-initial');

    expect(afterTimelineClose.activeColumnId).toBe('profile-1');
    expect(afterTimelineClose.columns).toHaveLength(1);
  });
});
