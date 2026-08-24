import { useEffect, useRef } from 'react';
import { useShallow } from 'zustand/react/shallow';

import type { GameRoomView } from '@/lib/api';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import {
  activateColumn,
  columnIdentityId,
  openTransientColumn,
  setColumnTimelineView,
  type ColumnKind,
  type ColumnTimelineView,
} from '@/shell/slices/workspace';

export function useDesktopShellColumnSynchronization(activeGameRooms: GameRoomView[]) {
  const {
    activeGamePanelStatus,
    activePrimarySection,
    activeTopic,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedDirectMessagePeerPubkey,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
    timelineView,
  } = useDesktopShellStore(
    useShallow((state) => ({
      // game rooms の readiness 信号(loadGameSection が ready / error を書き込む)。
      activeGamePanelStatus: state.gamePanelStateByTopic[state.activeTopic]?.status,
      activePrimarySection: state.shellChromeState.activePrimarySection,
      activeTopic: state.activeTopic,
      selectedAuthorPubkey: state.selectedAuthorPubkey,
      selectedChannelIdByTopic: state.selectedChannelIdByTopic,
      selectedDirectMessagePeerPubkey: state.selectedDirectMessagePeerPubkey,
      selectedGameRoomId: state.selectedGameRoomId,
      selectedLiveSessionId: state.selectedLiveSessionId,
      selectedThread: state.selectedThread,
      timelineView: state.shellChromeState.timelineView,
    }))
  );
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');
  // route(chrome projection)由来の timelineView を Column に反映するための遷移検出。
  // 初回実行(prev === null)で 'feed' を適用しないのは、persistence 復元値を
  // timelineView query の無い hash で上書きしないため(Issue #765)。
  const previousTimelineViewRef = useRef<ColumnTimelineView | null>(null);

  useEffect(() => {
    const scope = {
      topicId: activeTopic,
      channelId: selectedChannelIdByTopic[activeTopic] ?? null,
    };
    const previousTimelineView = previousTimelineViewRef.current;
    previousTimelineViewRef.current = timelineView;
    // deep link(timelineView=bookmarks)は常に反映する。feed への反映は route 遷移
    // (bookmarks → feed)を観測した場合のみ行い、復元値の維持と両立させる。
    const applyTimelineView =
      timelineView === 'bookmarks' ||
      (previousTimelineView !== null && previousTimelineView !== timelineView);
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
      // Stream / Game / Metaverse は独立 transient(親なし)として開く(Issue #765)。
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
          break;
        case 'game': {
          const selectedRoom = activeGameRooms.find(
            (room) => room.room_id === selectedGameRoomId
          );
          if (
            selectedGameRoomId &&
            !selectedRoom &&
            activeGamePanelStatus !== 'ready' &&
            activeGamePanelStatus !== 'error'
          ) {
            // rooms が未ロードのうちは kind(game / metaverse)を確定できないため Column を作らない。
            // ロード完了(ready / error)で effect が再実行され、確定した kind で開く。
            return;
          }
          kind = selectedRoom?.room_kind === 'score_game' ? 'game' : 'metaverse';
          entityId = selectedGameRoomId ?? undefined;
          break;
        }
      }
    }

    setWorkspaceState((incoming) => {
      const timelineId = columnIdentityId('timeline', scope);
      // route の focus 対象 scope の Timeline Column へ view を投影する。
      // 対象 Column が存在しない場合は no-op(setColumnTimelineView 側で吸収)。
      const current = applyTimelineView
        ? setColumnTimelineView(incoming, timelineId, timelineView)
        : incoming;
      const ensureTimelineColumn = (state: typeof current) => {
        if (state.columns.some((column) => column.id === timelineId)) return state;
        return openTransientColumn(state, {
          id: timelineId,
          kind: 'timeline',
          scope,
          pinned: false,
        });
      };
      if (selectedAuthorPubkey && selectedDirectMessagePeerPubkey) {
        const messagesId = columnIdentityId('messages', scope);
        let next = openTransientColumn(current, {
          id: messagesId,
          kind: 'messages',
          scope,
          pinned: false,
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
        });
        return openTransientColumn(next, {
          id: columnIdentityId('profile', scope, selectedAuthorPubkey),
          kind: 'profile',
          scope,
          entityId: selectedAuthorPubkey,
          parentColumnId: conversationId,
          pinned: false,
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
        });
        next = openTransientColumn(next, {
          id: columnIdentityId('profile', scope, selectedAuthorPubkey),
          kind: 'profile',
          scope,
          entityId: selectedAuthorPubkey,
          parentColumnId: threadId,
          pinned: false,
        });
        return next;
      }
      if (kind === 'timeline') {
        const next = ensureTimelineColumn(current);
        return activateColumn(next, timelineId);
      }
      const id = columnIdentityId(kind, scope, entityId);
      // 対象 Column 自身が active のまま effect が再実行された場合(header 再アクティブ化など)は、
      // activeColumnId を parent に採ると自己参照になるため、既存の parent を維持する。
      const existingParentColumnId = current.columns.find((column) => column.id === id)
        ?.parentColumnId;
      const parentColumnId =
        kind === 'thread'
          ? timelineId
          : !childColumn
            ? undefined
            : current.activeColumnId === id
              ? existingParentColumnId
              : current.activeColumnId;
      const base = kind === 'thread' ? ensureTimelineColumn(current) : current;
      return openTransientColumn(base, {
        id,
        kind,
        scope,
        entityId,
        parentColumnId,
        pinned: false,
      });
    });
  }, [
    activeGamePanelStatus,
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
    timelineView,
  ]);
}
