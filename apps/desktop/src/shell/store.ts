import { createContext, useCallback, useContext } from 'react';
import { useStore } from 'zustand';
import { createStore } from 'zustand/vanilla';

import { type DesktopApi } from '@/lib/api';
import type { DesktopTheme } from '@/lib/theme';
import { type ChromeSliceState, createInitialChromeSlice } from '@/shell/slices/chrome';
import {
  type ColumnDraftsSliceState,
  createInitialColumnDraftsSlice,
} from '@/shell/slices/columnDrafts';
import {
  type ConnectivitySliceState,
  createInitialConnectivitySlice,
} from '@/shell/slices/connectivity';
import {
  type DirectMessagesSliceState,
  createInitialDirectMessagesSlice,
} from '@/shell/slices/directMessages';
import { type LiveGameSliceState, createInitialLiveGameSlice } from '@/shell/slices/liveGame';
import { type MediaSliceState, createInitialMediaSlice } from '@/shell/slices/media';
import {
  type NotificationsSliceState,
  createInitialNotificationsSlice,
} from '@/shell/slices/notifications';
import {
  type PrivateChannelSliceState,
  createInitialPrivateChannelSlice,
} from '@/shell/slices/privateChannel';
import {
  type ProfileSocialSliceState,
  createInitialProfileSocialSlice,
} from '@/shell/slices/profileSocial';
import {
  type ReactionsBookmarksSliceState,
  createInitialReactionsBookmarksSlice,
} from '@/shell/slices/reactionsBookmarks';
import { type TimelineSliceState, createInitialTimelineSlice } from '@/shell/slices/timeline';
import {
  type WorkspaceSliceState,
  createInitialWorkspaceSlice,
} from '@/shell/slices/workspace';
import {
  readWorkspaceLayout,
  type WorkspaceStorage,
} from '@/shell/workspacePersistence';
import { readSavedWorkspaceLayouts } from '@/shell/savedWorkspaceLayouts';
import {
  readColumnDrafts,
  type ColumnDraftStorage,
} from '@/shell/columnDraftPersistence';
import {
  readCommunityIndexNodePreference,
  type CommunityIndexNodePreferenceStorage,
} from '@/shell/communityIndexNodePreference';

export type AppProps = {
  api?: DesktopApi;
};

// 状態の定義はドメインスライス(@/shell/slices/)へ分割した(WP-H6 PR3)。
// ここでは合成のみ行う。**実行時は従来どおり単一 store** であり、1 回の set() は
// スライスをまたいでも原子的(useRouteSynchronization の一括更新が壊れない)。
export type DesktopShellState = TimelineSliceState &
  PrivateChannelSliceState &
  ConnectivitySliceState &
  MediaSliceState &
  ProfileSocialSliceState &
  ReactionsBookmarksSliceState &
  NotificationsSliceState &
  DirectMessagesSliceState &
  LiveGameSliceState &
  ChromeSliceState &
  ColumnDraftsSliceState &
  WorkspaceSliceState;

export type DesktopShellStateValue<K extends keyof DesktopShellState> =
  | DesktopShellState[K]
  | ((current: DesktopShellState[K]) => DesktopShellState[K]);

export type DesktopShellStore = DesktopShellState & {
  patchState: (patch: Partial<DesktopShellState>) => void;
  resetState: () => void;
  setField: <K extends keyof DesktopShellState>(
    key: K,
    value: DesktopShellStateValue<K>
  ) => void;
};

export type DesktopShellPageProps = AppProps & {
  theme: DesktopTheme;
  onThemeChange: (theme: DesktopTheme) => void;
};

export const REFRESH_INTERVAL_MS = 3000;
export const CONNECTIVITY_STATUS_FALLBACK_INTERVAL_MS = 60000;
export const STATUS_REFRESH_INTERVAL_MS = 60000;
export const VIDEO_POSTER_TIMEOUT_MS = 5000;
export const MEDIA_DEBUG_STORAGE_KEY = 'kukuri:media-debug';
export const SHELL_WORKSPACE_ID = 'shell-primary-workspace';
export const SHELL_SETTINGS_ID = 'shell-settings-drawer';

export function createInitialShellState(): DesktopShellState {
  const timeline = createInitialTimelineSlice();
  const privateChannels = createInitialPrivateChannelSlice();
  return {
    ...timeline,
    ...privateChannels,
    ...createInitialConnectivitySlice(),
    ...createInitialMediaSlice(),
    ...createInitialProfileSocialSlice(),
    ...createInitialReactionsBookmarksSlice(),
    ...createInitialNotificationsSlice(),
    ...createInitialDirectMessagesSlice(),
    ...createInitialLiveGameSlice(),
    ...createInitialChromeSlice(),
    ...createInitialColumnDraftsSlice(),
    ...createInitialWorkspaceSlice({
      topicId: timeline.trackedTopics[0],
      channelId: null,
    }),
  };
}

type CreateDesktopShellStoreOptions = {
  workspaceStorage?: WorkspaceStorage;
  draftStorage?: ColumnDraftStorage;
  communityIndexPreferenceStorage?: CommunityIndexNodePreferenceStorage;
};

export function createDesktopShellStore(options: CreateDesktopShellStoreOptions = {}) {
  const initialState = createInitialShellState();
  const workspaceState = options.workspaceStorage
    ? readWorkspaceLayout(options.workspaceStorage, initialState.workspaceState)
    : initialState.workspaceState;
  const savedWorkspaceLayouts = options.workspaceStorage
    ? readSavedWorkspaceLayouts(options.workspaceStorage)
    : initialState.savedWorkspaceLayouts;
  const normalizedWorkspaceState =
    workspaceState.activeLayoutId &&
    !savedWorkspaceLayouts.some((layout) => layout.id === workspaceState.activeLayoutId)
      ? { ...workspaceState, activeLayoutId: null }
      : workspaceState;
  const columnDraftsByKey = options.draftStorage
    ? readColumnDrafts(options.draftStorage)
    : initialState.columnDraftsByKey;
  const communityIndexNodePreference = options.communityIndexPreferenceStorage
    ? readCommunityIndexNodePreference(options.communityIndexPreferenceStorage)
    : initialState.communityIndexNodePreference;
  return createStore<DesktopShellStore>((set) => ({
    ...initialState,
    workspaceState: normalizedWorkspaceState,
    savedWorkspaceLayouts,
    columnDraftsByKey,
    communityIndexNodePreference,
    patchState: (patch) => set((current) => ({ ...current, ...patch })),
    resetState: () => set(createInitialShellState()),
    setField: (key, value) =>
      set((current) => {
        const nextValue =
          typeof value === 'function'
            ? (value as (currentValue: DesktopShellState[typeof key]) => DesktopShellState[typeof key])(
                current[key]
              )
            : value;
        if (Object.is(current[key], nextValue)) {
          return current;
        }
        return {
          [key]: nextValue,
        };
      }),
  }));
}

export type DesktopShellStoreApi = ReturnType<typeof createDesktopShellStore>;

export const DesktopShellStoreContext = createContext<DesktopShellStoreApi | null>(null);

export function useDesktopShellStoreApi() {
  const store = useContext(DesktopShellStoreContext);
  if (!store) {
    throw new Error('desktop shell store is not available');
  }

  return store;
}

export function useDesktopShellStore(): DesktopShellStore;
export function useDesktopShellStore<T>(selector: (state: DesktopShellStore) => T): T;
export function useDesktopShellStore<T>(selector?: (state: DesktopShellStore) => T) {
  const resolvedSelector =
    selector ?? ((state: DesktopShellStore) => state as unknown as T);
  return useStore(
    useDesktopShellStoreApi(),
    resolvedSelector
  );
}

export function useDesktopShellFieldSetter<K extends keyof DesktopShellState>(key: K) {
  const setField = useDesktopShellStore((state) => state.setField);

  return useCallback(
    (value: DesktopShellStateValue<K>) => {
      setField(key, value);
    },
    [key, setField]
  );
}

// 既存の import 面の互換 re-export(型・定数はスライス側が定義元)。
export {
  DEFAULT_ASYNC_PANEL_STATE,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  activeTimelineStorageKey,
  timelineScopeStorageKey,
  timelineStorageKeyForChannel,
} from '@/shell/slices/shared';
export type { AsyncPanelState, DraftMediaItem } from '@/shell/slices/shared';
export { DEFAULT_COMMUNITY_NODE_CONFIG } from '@/shell/slices/connectivity';
export type {
  CommunityNodeDraftNode,
  CommunityNodeManifestEntry,
  CommunityNodePoliciesEntry,
} from '@/shell/slices/connectivity';
export type { KnownAuthorsByPubkey } from '@/shell/slices/profileSocial';
export type { GameEditorDraft } from '@/shell/slices/liveGame';
