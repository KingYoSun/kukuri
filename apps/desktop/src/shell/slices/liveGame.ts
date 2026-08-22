import type { GameRoomStatus, GameRoomView, LiveSessionView } from '@/lib/api';

import {
  type AsyncPanelState,
  DEFAULT_ASYNC_PANEL_STATE,
  buildStarterTopicRecord,
} from '@/shell/slices/shared';

/// ライブセッション・ゲームルーム(WP-H6 PR3 のドメインスライス)。

export type GameEditorDraft = {
  status: GameRoomStatus;
  phase_label: string;
  scores: Record<string, string>;
};

export type LiveGameSliceState = {
  liveSessionsByTopic: Record<string, LiveSessionView[]>;
  liveSessionsByScopeKey: Record<string, LiveSessionView[]>;
  gameRoomsByTopic: Record<string, GameRoomView[]>;
  gameRoomsByScopeKey: Record<string, GameRoomView[]>;
  liveTitle: string;
  liveDescription: string;
  liveError: string | null;
  livePanelStateByTopic: Record<string, AsyncPanelState>;
  livePanelStateByScopeKey: Record<string, AsyncPanelState>;
  liveCreatePending: boolean;
  livePendingBySessionId: Record<string, true>;
  selectedLiveSessionId: string | null;
  gameTitle: string;
  gameDescription: string;
  gameParticipantsInput: string;
  gameError: string | null;
  gameDrafts: Record<string, GameEditorDraft>;
  gamePanelStateByTopic: Record<string, AsyncPanelState>;
  gamePanelStateByScopeKey: Record<string, AsyncPanelState>;
  gameCreatePending: boolean;
  gameSavingByRoomId: Record<string, true>;
  selectedGameRoomId: string | null;
};

export function createInitialLiveGameSlice(): LiveGameSliceState {
  return {
    liveSessionsByTopic: buildStarterTopicRecord(() => [] as LiveSessionView[]),
    liveSessionsByScopeKey: {},
    gameRoomsByTopic: buildStarterTopicRecord(() => [] as GameRoomView[]),
    gameRoomsByScopeKey: {},
    liveTitle: '',
    liveDescription: '',
    liveError: null,
    livePanelStateByTopic: buildStarterTopicRecord(() => ({ ...DEFAULT_ASYNC_PANEL_STATE })),
    livePanelStateByScopeKey: {},
    liveCreatePending: false,
    livePendingBySessionId: {},
    selectedLiveSessionId: null,
    gameTitle: '',
    gameDescription: '',
    gameParticipantsInput: '',
    gameError: null,
    gameDrafts: {},
    gamePanelStateByTopic: buildStarterTopicRecord(() => ({ ...DEFAULT_ASYNC_PANEL_STATE })),
    gamePanelStateByScopeKey: {},
    gameCreatePending: false,
    gameSavingByRoomId: {},
    selectedGameRoomId: null,
  };
}
