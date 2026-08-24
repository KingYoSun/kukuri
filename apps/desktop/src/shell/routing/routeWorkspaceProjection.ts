import type { PrimarySection } from '@/components/shell/types';
import {
  activateColumn,
  columnIdentityId,
  openTransientColumn,
  setColumnTimelineView,
  type ColumnKind,
  type ColumnScope,
  type ColumnTimelineView,
  type WorkspaceState,
} from '@/shell/slices/workspace';

type RouteWorkspaceProjection = {
  gameColumnResolutionPending: boolean;
  isScoreGameRoom: boolean;
  nextTimelineView: ColumnTimelineView;
  routeScope: ColumnScope;
  routeSection: PrimarySection;
  selectedAuthorPubkey: string | null;
  selectedDirectMessagePeerPubkey: string | null;
  selectedGameRoomId: string | null;
  selectedLiveSessionId: string | null;
  selectedThread: string | null;
};

export function workspaceForRoute(
  incoming: WorkspaceState,
  projection: RouteWorkspaceProjection
): WorkspaceState {
  const {
    gameColumnResolutionPending,
    isScoreGameRoom,
    nextTimelineView,
    routeScope,
    routeSection,
    selectedAuthorPubkey,
    selectedDirectMessagePeerPubkey,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
  } = projection;
  const timelineId = columnIdentityId('timeline', routeScope);
  const ensureTimelineColumn = (current: WorkspaceState) => {
    const withView = setColumnTimelineView(current, timelineId, nextTimelineView);
    if (withView.columns.some((column) => column.id === timelineId)) return withView;
    return openTransientColumn(withView, {
      id: timelineId,
      kind: 'timeline',
      scope: routeScope,
      pinned: false,
      timelineView: nextTimelineView,
    });
  };

  if (selectedAuthorPubkey && selectedDirectMessagePeerPubkey) {
    const profileId = columnIdentityId('profile', routeScope, selectedAuthorPubkey);
    if (incoming.activeColumnId === profileId) return incoming;
    const messagesId = columnIdentityId('messages', routeScope);
    let next = openTransientColumn(incoming, {
      id: messagesId,
      kind: 'messages',
      scope: routeScope,
      pinned: false,
    });
    const conversationId = columnIdentityId(
      'conversation',
      routeScope,
      selectedDirectMessagePeerPubkey
    );
    next = openTransientColumn(next, {
      id: conversationId,
      kind: 'conversation',
      scope: routeScope,
      entityId: selectedDirectMessagePeerPubkey,
      parentColumnId: messagesId,
      pinned: false,
    });
    return openTransientColumn(next, {
      id: profileId,
      kind: 'profile',
      scope: routeScope,
      entityId: selectedAuthorPubkey,
      parentColumnId: conversationId,
      pinned: false,
    });
  }

  if (selectedThread) {
    const threadId = columnIdentityId('thread', routeScope, selectedThread);
    const activeRouteColumnId = selectedAuthorPubkey
      ? columnIdentityId('profile', routeScope, selectedAuthorPubkey)
      : threadId;
    if (incoming.activeColumnId === activeRouteColumnId) return incoming;
    let next = ensureTimelineColumn(incoming);
    next = openTransientColumn(next, {
      id: threadId,
      kind: 'thread',
      scope: routeScope,
      entityId: selectedThread,
      parentColumnId: timelineId,
      pinned: false,
    });
    if (!selectedAuthorPubkey) return next;
    return openTransientColumn(next, {
      id: columnIdentityId('profile', routeScope, selectedAuthorPubkey),
      kind: 'profile',
      scope: routeScope,
      entityId: selectedAuthorPubkey,
      parentColumnId: threadId,
      pinned: false,
    });
  }

  if (selectedDirectMessagePeerPubkey) {
    const conversationId = columnIdentityId(
      'conversation',
      routeScope,
      selectedDirectMessagePeerPubkey
    );
    if (incoming.activeColumnId === conversationId) return incoming;
    const messagesId = columnIdentityId('messages', routeScope);
    const next = openTransientColumn(incoming, {
      id: messagesId,
      kind: 'messages',
      scope: routeScope,
      pinned: false,
    });
    return openTransientColumn(next, {
      id: conversationId,
      kind: 'conversation',
      scope: routeScope,
      entityId: selectedDirectMessagePeerPubkey,
      parentColumnId: messagesId,
      pinned: false,
    });
  }

  if (selectedAuthorPubkey) {
    const id = columnIdentityId('profile', routeScope, selectedAuthorPubkey);
    if (incoming.activeColumnId === id) return incoming;
    return openTransientColumn(incoming, {
      id,
      kind: 'profile',
      scope: routeScope,
      entityId: selectedAuthorPubkey,
      parentColumnId: incoming.activeColumnId,
      pinned: false,
    });
  }

  let routeColumnKind: ColumnKind;
  let routeEntityId: string | undefined;
  if (routeSection === 'notifications') {
    routeColumnKind = 'notifications';
  } else if (routeSection === 'messages') {
    routeColumnKind = 'messages';
  } else if (routeSection === 'profile') {
    routeColumnKind = 'profile';
  } else if (routeSection === 'explore') {
    routeColumnKind = 'explore';
  } else if (routeSection === 'live') {
    routeColumnKind = 'stream';
    routeEntityId = selectedLiveSessionId ?? undefined;
  } else if (routeSection === 'game') {
    routeColumnKind = isScoreGameRoom || gameColumnResolutionPending ? 'game' : 'metaverse';
    routeEntityId = selectedGameRoomId ?? undefined;
  } else {
    return activateColumn(ensureTimelineColumn(incoming), timelineId);
  }
  const routeColumnId = columnIdentityId(routeColumnKind, routeScope, routeEntityId);
  if (incoming.activeColumnId === routeColumnId) return incoming;
  return openTransientColumn(incoming, {
    id: routeColumnId,
    kind: routeColumnKind,
    scope: routeScope,
    entityId: routeEntityId,
    pinned: false,
  });
}
