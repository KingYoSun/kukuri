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
import { setColumnDraft } from '@/shell/slices/columnDrafts';
import { columnIdentityId } from '@/shell/slices/workspace';

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

function createReviewStore() {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'true');
  window.history.replaceState(
    null,
    '',
    '#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-review'
  );
  const store = createDesktopShellStore();
  const publicColumnId = columnIdentityId('timeline', DEMO_SCOPE);
  const friendsColumnId = columnIdentityId('timeline', FRIENDS_SCOPE);
  const irohColumnId = columnIdentityId('timeline', IROH_SCOPE);
  store.setState((current) => ({
    activeTopic: DEMO_SCOPE.topicId,
    selectedChannelIdByTopic: {
      ...current.selectedChannelIdByTopic,
      [DEMO_SCOPE.topicId]: FRIENDS_SCOPE.channelId,
    },
    joinedChannelsByTopic: {
      ...current.joinedChannelsByTopic,
      [DEMO_SCOPE.topicId]: [REVIEW_CHANNEL],
    },
    workspaceState: {
      ...current.workspaceState,
      activeColumnId: friendsColumnId,
      columns: [
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

export function ProductionColumnWorkspaceStory() {
  const [store] = useState(createReviewStore);
  const api = useMemo(() => {
    const mock = createDesktopMockApi();
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
