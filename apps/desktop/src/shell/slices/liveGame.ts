import type { GameRoomStatus, GameRoomView, LiveSessionView } from '@/lib/api';

import type { AsyncPanelState } from '@/shell/slices/shared';

/// ライブセッション・ゲームルーム(WP-H6 PR3 のドメインスライス)。

export type GameEditorDraft = {
  status: GameRoomStatus;
  phase_label: string;
  scores: Record<string, string>;
};

export type LiveGameSliceState = {
  liveSessionsByScopeKey: Record<string, LiveSessionView[]>;
  gameRoomsByScopeKey: Record<string, GameRoomView[]>;
  liveTitle: string;
  liveDescription: string;
  liveError: string | null;
  livePanelStateByScopeKey: Record<string, AsyncPanelState>;
  liveCreatePending: boolean;
  livePendingBySessionId: Record<string, true>;
  selectedLiveSessionId: string | null;
  gameTitle: string;
  gameDescription: string;
  gameParticipantsInput: string;
  gameError: string | null;
  gameDrafts: Record<string, GameEditorDraft>;
  gamePanelStateByScopeKey: Record<string, AsyncPanelState>;
  gameCreatePending: boolean;
  gameSavingByRoomId: Record<string, true>;
  selectedGameRoomId: string | null;
};

export function createInitialLiveGameSlice(): LiveGameSliceState {
  return {
    liveSessionsByScopeKey: {},
    gameRoomsByScopeKey: {},
    liveTitle: '',
    liveDescription: '',
    liveError: null,
    livePanelStateByScopeKey: {},
    liveCreatePending: false,
    livePendingBySessionId: {},
    selectedLiveSessionId: null,
    gameTitle: '',
    gameDescription: '',
    gameParticipantsInput: '',
    gameError: null,
    gameDrafts: {},
    gamePanelStateByScopeKey: {},
    gameCreatePending: false,
    gameSavingByRoomId: {},
    selectedGameRoomId: null,
  };
}
