import { useMemo } from 'react';

import type {
  ChannelAudienceOption,
  GameDraftView,
  PrivateChannelListItemView,
} from '@/components/extended/types';
import type { GameRoomView, LiveSessionView } from '@/lib/api';
import { createGameEditorDraft } from '@/shell/presentation';
import type { DesktopShellState } from '@/shell/store';

type UseChannelsViewModelsArgs = {
  activeGameRooms: GameRoomView[];
  activeJoinedChannels: DesktopShellState['joinedChannelsByTopic'][string];
  activeLiveSessions: LiveSessionView[];
  gameDrafts: DesktopShellState['gameDrafts'];
  livePendingBySessionId: DesktopShellState['livePendingBySessionId'];
  localAuthorPubkey: string;
  selectedPrivateChannelId: string | null;
  t: (key: string, options?: Record<string, unknown>) => string;
};

// channels / live / game section の projection(Q3 T5 の分割先)。
export function useChannelsViewModels({
  activeGameRooms,
  activeJoinedChannels,
  activeLiveSessions,
  gameDrafts,
  livePendingBySessionId,
  localAuthorPubkey,
  selectedPrivateChannelId,
  t,
}: UseChannelsViewModelsArgs) {
  const channelAudienceOptions = useMemo<ChannelAudienceOption[]>(
    () => [
      {
        value: 'invite_only',
        label: t('channels:audienceOptions.invite_only'),
      },
      {
        value: 'friend_only',
        label: t('channels:audienceOptions.friend_only'),
      },
      {
        value: 'friend_plus',
        label: t('channels:audienceOptions.friend_plus'),
      },
    ],
    [t]
  );

  const privateChannelListItems = useMemo<PrivateChannelListItemView[]>(
    () =>
      activeJoinedChannels.map((channel) => ({
        channel,
        active: channel.channel_id === selectedPrivateChannelId,
      })),
    [activeJoinedChannels, selectedPrivateChannelId]
  );

  const liveSessionListItems = useMemo(
    () =>
      activeLiveSessions.map((session) => ({
        session,
        isOwner: session.host_pubkey === localAuthorPubkey,
        pending: Boolean(livePendingBySessionId[session.session_id]),
      })),
    [activeLiveSessions, livePendingBySessionId, localAuthorPubkey]
  );

  const gameDraftViews = useMemo<Record<string, GameDraftView>>(
    () =>
      Object.fromEntries(
        activeGameRooms.map((room) => {
          const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
          return [
            room.room_id,
            {
              status: draft.status,
              phaseLabel: draft.phase_label,
              scores: draft.scores,
            },
          ];
        })
      ),
    [activeGameRooms, gameDrafts]
  );

  return {
    channelAudienceOptions,
    privateChannelListItems,
    liveSessionListItems,
    gameDraftViews,
  };
}
