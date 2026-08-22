import { useEffect } from 'react';
import { useShallow } from 'zustand/react/shallow';

import type { GameRoomView } from '@/lib/api';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import {
  activateColumn,
  columnIdentityId,
  openTransientColumn,
  type ColumnKind,
} from '@/shell/slices/workspace';

export function useDesktopShellColumnSynchronization(activeGameRooms: GameRoomView[]) {
  const {
    activePrimarySection,
    activeTopic,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedDirectMessagePeerPubkey,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
  } = useDesktopShellStore(
    useShallow((state) => ({
      activePrimarySection: state.shellChromeState.activePrimarySection,
      activeTopic: state.activeTopic,
      selectedAuthorPubkey: state.selectedAuthorPubkey,
      selectedChannelIdByTopic: state.selectedChannelIdByTopic,
      selectedDirectMessagePeerPubkey: state.selectedDirectMessagePeerPubkey,
      selectedGameRoomId: state.selectedGameRoomId,
      selectedLiveSessionId: state.selectedLiveSessionId,
      selectedThread: state.selectedThread,
    }))
  );
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');

  useEffect(() => {
    const scope = {
      topicId: activeTopic,
      channelId: selectedChannelIdByTopic[activeTopic] ?? null,
    };
    let kind: ColumnKind;
    let entityId: string | undefined;
    let childColumn = false;

    if (selectedAuthorPubkey) {
      kind = 'profile';
      entityId = selectedAuthorPubkey;
      childColumn = true;
    } else if (selectedThread) {
      kind = 'thread';
      entityId = selectedThread;
      childColumn = true;
    } else if (activePrimarySection === 'messages' && selectedDirectMessagePeerPubkey) {
      kind = 'conversation';
      entityId = selectedDirectMessagePeerPubkey;
      childColumn = true;
    } else {
      switch (activePrimarySection) {
        case 'timeline':
          kind = 'timeline';
          break;
        case 'notifications':
          kind = 'notifications';
          break;
        case 'explore':
          kind = 'explore';
          break;
        case 'messages':
          kind = 'messages';
          break;
        case 'profile':
          kind = 'profile';
          break;
        case 'live':
          kind = 'stream';
          entityId = selectedLiveSessionId ?? undefined;
          childColumn = Boolean(entityId);
          break;
        case 'game': {
          const selectedRoom = activeGameRooms.find(
            (room) => room.room_id === selectedGameRoomId
          );
          kind = selectedRoom?.room_kind === 'score_game' ? 'game' : 'metaverse';
          entityId = selectedGameRoomId ?? undefined;
          childColumn = Boolean(entityId);
          break;
        }
      }
    }

    setWorkspaceState((current) => {
      const timelineId = columnIdentityId('timeline', scope);
      const ensureTimelineColumn = (state: typeof current) => {
        if (state.columns.some((column) => column.id === timelineId)) return state;
        return openTransientColumn(state, {
          id: timelineId,
          kind: 'timeline',
          scope,
          pinned: false,
          preferredDesktopSpan: 1,
        });
      };
      if (selectedAuthorPubkey && selectedDirectMessagePeerPubkey) {
        const messagesId = columnIdentityId('messages', scope);
        let next = openTransientColumn(current, {
          id: messagesId,
          kind: 'messages',
          scope,
          pinned: false,
          preferredDesktopSpan: 1,
        });
        const conversationId = columnIdentityId(
          'conversation',
          scope,
          selectedDirectMessagePeerPubkey
        );
        next = openTransientColumn(next, {
          id: conversationId,
          kind: 'conversation',
          scope,
          entityId: selectedDirectMessagePeerPubkey,
          parentColumnId: messagesId,
          pinned: false,
          preferredDesktopSpan: 1,
        });
        return openTransientColumn(next, {
          id: columnIdentityId('profile', scope, selectedAuthorPubkey),
          kind: 'profile',
          scope,
          entityId: selectedAuthorPubkey,
          parentColumnId: conversationId,
          pinned: false,
          preferredDesktopSpan: 1,
        });
      }
      if (selectedAuthorPubkey && selectedThread) {
        const threadId = columnIdentityId('thread', scope, selectedThread);
        let next = ensureTimelineColumn(current);
        next = openTransientColumn(next, {
          id: threadId,
          kind: 'thread',
          scope,
          entityId: selectedThread,
          parentColumnId: timelineId,
          pinned: false,
          preferredDesktopSpan: 1,
        });
        next = openTransientColumn(next, {
          id: columnIdentityId('profile', scope, selectedAuthorPubkey),
          kind: 'profile',
          scope,
          entityId: selectedAuthorPubkey,
          parentColumnId: threadId,
          pinned: false,
          preferredDesktopSpan: 1,
        });
        return next;
      }
      if (kind === 'timeline') {
        const next = ensureTimelineColumn(current);
        return activateColumn(next, timelineId);
      }
      const id = columnIdentityId(kind, scope, entityId);
      const parentColumnId =
        kind === 'thread' ? timelineId : childColumn ? current.activeColumnId : undefined;
      const base = kind === 'thread' ? ensureTimelineColumn(current) : current;
      return openTransientColumn(base, {
        id,
        kind,
        scope,
        entityId,
        parentColumnId,
        pinned: false,
        preferredDesktopSpan: 1,
      });
    });
  }, [
    activeGameRooms,
    activePrimarySection,
    activeTopic,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedDirectMessagePeerPubkey,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
    setWorkspaceState,
  ]);
}
