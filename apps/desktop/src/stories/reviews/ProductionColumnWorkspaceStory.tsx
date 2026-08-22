import { useMemo, useState } from 'react';

import { DesktopShellPage } from '@/shell/DesktopShellPage';
import {
  createDesktopShellStore,
  DesktopShellStoreContext,
} from '@/shell/store';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { DEVELOPER_MODE_STORAGE_KEY } from '@/lib/developerMode';
import type { DesktopTheme } from '@/lib/theme';
import type { JoinedPrivateChannelView } from '@/lib/api';
import type { GameRoomView, LiveSessionView } from '@/lib/api';
import { DEFAULT_SHARED_OBJECT } from '@/components/extended/MetaverseSceneModel';
import { setColumnDraft } from '@/shell/slices/columnDrafts';
import { columnIdentityId, defaultColumnSpan } from '@/shell/slices/workspace';

const DEMO_SCOPE = { topicId: 'kukuri:topic:demo', channelId: null };
const FRIENDS_SCOPE = { topicId: 'kukuri:topic:demo', channelId: 'channel-review' };
const IROH_SCOPE = { topicId: 'kukuri:topic:iroh', channelId: null };
const REVIEW_CHANNEL: JoinedPrivateChannelView = {
  topic_id: DEMO_SCOPE.topicId,
  channel_id: FRIENDS_SCOPE.channelId,
  label: 'Core friends',
  creator_pubkey: 'f'.repeat(64),
  owner_pubkey: 'f'.repeat(64),
  joined_via_pubkey: null,
  audience_kind: 'invite_only',
  is_owner: true,
  current_epoch_id: 'review-epoch',
  archived_epoch_ids: [],
  sharing_state: 'open',
  rotation_required: false,
  participant_count: 4,
  stale_participant_count: 0,
};

type ProductionColumnWorkspaceStoryProps = {
  initialControlCenterOpen?: boolean;
  scenario?: 'scoped-drafts' | 'wide-surfaces';
  metaverseSpan?: 3 | 4;
};

const REVIEW_TIMESTAMP = 1_787_420_800_000;
const REVIEW_LIVE_SESSION: LiveSessionView = {
  session_id: 'live-review',
  host_pubkey: 'f'.repeat(64),
  title: 'Wave 5 launch stream',
  description: 'A production Stream Column using its two-span layout.',
  status: 'Live',
  started_at: REVIEW_TIMESTAMP,
  ended_at: null,
  viewer_count: 18,
  joined_by_me: true,
  channel_id: null,
  audience_label: 'Public',
};
const REVIEW_METAVERSE_ROOM: GameRoomView = {
  room_id: 'metaverse-review',
  host_pubkey: 'f'.repeat(64),
  title: 'Wave 5 atrium',
  description: 'A production Metaverse Column with a container-aware HUD.',
  status: 'Waiting',
  phase_label: 'metaverse-mvp',
  scores: [],
  room_kind: 'metaverse_room',
  metaverse: {
    world_version: 1,
    max_peers: 8,
    scene: { ground: 'default', shared_object: DEFAULT_SHARED_OBJECT },
    default_spawn: { position: [0, 0, 260], rotation: [0, 180, 0] },
    asset_refs: [],
    chat_history: [],
  },
  manifest_blob_hash: 'mock-metaverse-review',
  updated_at: REVIEW_TIMESTAMP,
  channel_id: null,
  audience_label: 'Public',
};

function createReviewStore(
  initialControlCenterOpen = false,
  scenario: ProductionColumnWorkspaceStoryProps['scenario'] = 'scoped-drafts',
  metaverseSpan: 3 | 4 = 3
) {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'true');
  window.history.replaceState(null, '', scenario === 'wide-surfaces'
    ? '#/game?topic=kukuri%3Atopic%3Ademo&roomId=metaverse-review'
    : '#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-review');
  const store = createDesktopShellStore();
  const publicColumnId = columnIdentityId('timeline', DEMO_SCOPE);
  const friendsColumnId = columnIdentityId('timeline', FRIENDS_SCOPE);
  const irohColumnId = columnIdentityId('timeline', IROH_SCOPE);
  const streamColumnId = columnIdentityId('stream', DEMO_SCOPE, REVIEW_LIVE_SESSION.session_id);
  const metaverseColumnId = columnIdentityId(
    'metaverse',
    DEMO_SCOPE,
    REVIEW_METAVERSE_ROOM.room_id
  );
  store.setState((current) => ({
    activeTopic: DEMO_SCOPE.topicId,
    selectedChannelIdByTopic: {
      ...current.selectedChannelIdByTopic,
      [DEMO_SCOPE.topicId]: scenario === 'wide-surfaces' ? null : FRIENDS_SCOPE.channelId,
    },
    shellChromeState: scenario === 'wide-surfaces'
      ? { ...current.shellChromeState, activePrimarySection: 'game' }
      : current.shellChromeState,
    joinedChannelsByTopic: {
      ...current.joinedChannelsByTopic,
      [DEMO_SCOPE.topicId]: [REVIEW_CHANNEL],
    },
    workspaceState: {
      ...current.workspaceState,
      activeColumnId: scenario === 'wide-surfaces' ? metaverseColumnId : friendsColumnId,
      controlCenterOpen: initialControlCenterOpen,
      columns: scenario === 'wide-surfaces' ? [
        {
          id: publicColumnId,
          kind: 'timeline',
          scope: DEMO_SCOPE,
          pinned: true,
          preferredDesktopSpan: defaultColumnSpan('timeline'),
        },
        {
          id: streamColumnId,
          kind: 'stream',
          scope: DEMO_SCOPE,
          entityId: REVIEW_LIVE_SESSION.session_id,
          pinned: true,
          preferredDesktopSpan: defaultColumnSpan('stream'),
        },
        {
          id: metaverseColumnId,
          kind: 'metaverse',
          scope: DEMO_SCOPE,
          entityId: REVIEW_METAVERSE_ROOM.room_id,
          pinned: true,
          preferredDesktopSpan: metaverseSpan,
        },
      ] : [
        {
          id: publicColumnId,
          kind: 'timeline',
          scope: DEMO_SCOPE,
          pinned: true,
          preferredDesktopSpan: 1,
        },
        {
          id: friendsColumnId,
          kind: 'timeline',
          scope: FRIENDS_SCOPE,
          pinned: true,
          preferredDesktopSpan: 1,
        },
        {
          id: irohColumnId,
          kind: 'timeline',
          scope: IROH_SCOPE,
          pinned: true,
          preferredDesktopSpan: 1,
        },
      ],
    },
    liveSessionsByTopic: {
      ...current.liveSessionsByTopic,
      [DEMO_SCOPE.topicId]: [REVIEW_LIVE_SESSION],
    },
    gameRoomsByTopic: {
      ...current.gameRoomsByTopic,
      [DEMO_SCOPE.topicId]: [REVIEW_METAVERSE_ROOM],
    },
    selectedGameRoomId:
      scenario === 'wide-surfaces' ? REVIEW_METAVERSE_ROOM.room_id : current.selectedGameRoomId,
    columnDraftsByKey: setColumnDraft(
      setColumnDraft(
        current.columnDraftsByKey,
        { columnId: publicColumnId, action: 'post', scope: DEMO_SCOPE },
        (draft) => ({ ...draft, content: 'Public launch note stays with this Column.' })
      ),
      { columnId: friendsColumnId, action: 'post', scope: FRIENDS_SCOPE },
      (draft) => ({
        ...draft,
        content: 'Friends-only release note — this Draft cannot move to Public.',
        expanded: true,
      })
    ),
  }));
  return store;
}

export function ProductionColumnWorkspaceStory({
  initialControlCenterOpen = false,
  scenario = 'scoped-drafts',
  metaverseSpan = 3,
}: ProductionColumnWorkspaceStoryProps) {
  const [store] = useState(() =>
    createReviewStore(initialControlCenterOpen, scenario, metaverseSpan)
  );
  const api = useMemo(() => {
    const mock = createDesktopMockApi({
      seedLiveSessions: { [DEMO_SCOPE.topicId]: [REVIEW_LIVE_SESSION] },
      seedGameRooms: { [DEMO_SCOPE.topicId]: [REVIEW_METAVERSE_ROOM] },
    });
    return {
      ...mock,
      listJoinedPrivateChannels: async (topic: string) =>
        topic === DEMO_SCOPE.topicId ? [REVIEW_CHANNEL] : [],
    };
  }, []);
  const [theme, setTheme] = useState<DesktopTheme>('dark');

  return (
    <DesktopShellStoreContext.Provider value={store}>
      <DesktopShellPage api={api} theme={theme} onThemeChange={setTheme} />
    </DesktopShellStoreContext.Provider>
  );
}
