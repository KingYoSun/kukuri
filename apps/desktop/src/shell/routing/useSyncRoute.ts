import { useCallback, type MutableRefObject } from 'react';
import type { NavigateFunction } from 'react-router-dom';

import {
  buildShellUrl,
  type DesktopShellRouteOverrides,
  type HashRouteLocation,
} from '@/shell/routes';
import type { DesktopShellStoreApi } from '@/shell/store';
import { routeStateForColumn } from '@/shell/routing/initialWorkspaceRoute';
import { activeWorkspaceColumn } from '@/shell/slices/workspace';

type UseSyncRouteArgs = {
  navigate: NavigateFunction;
  pendingRouteUrlRef: MutableRefObject<string | null>;
  resolvedRouteLocation: HashRouteLocation;
  storeApi: DesktopShellStoreApi;
};

export function useSyncRoute({
  navigate,
  pendingRouteUrlRef,
  resolvedRouteLocation,
  storeApi,
}: UseSyncRouteArgs) {
  return useCallback(
    (mode: 'push' | 'replace' = 'replace', overrides?: DesktopShellRouteOverrides) => {
      const currentState = storeApi.getState();
      const activeColumn = activeWorkspaceColumn(currentState.workspaceState);
      const columnRoute = routeStateForColumn(activeColumn);
      const hasOverride = <K extends keyof DesktopShellRouteOverrides>(key: K) =>
        overrides ? Object.prototype.hasOwnProperty.call(overrides, key) : false;
      const nextTopic = overrides?.activeTopic ?? columnRoute?.activeTopic ?? activeColumn.scope?.topicId;
      if (!nextTopic) return;
      const nextPrimarySection =
        overrides?.primarySection ?? columnRoute?.primarySection ?? 'timeline';
      const nextTimelineView =
        overrides?.timelineView ?? columnRoute?.timelineView ?? 'feed';
      const nextProfileMode =
        overrides?.profileMode ?? currentState.shellChromeState.profileMode;
      const nextProfileConnectionsView =
        overrides?.profileConnectionsView ?? currentState.shellChromeState.profileConnectionsView;
      const nextSelectedThread = hasOverride('selectedThread')
        ? overrides?.selectedThread ?? null
        : columnRoute?.selectedThread ?? currentState.selectedThread;
      const nextFocusedObjectId = hasOverride('focusedObjectId')
        ? overrides?.focusedObjectId ?? null
        : currentState.focusedObjectId;
      const nextSelectedAuthorPubkey = hasOverride('selectedAuthorPubkey')
        ? overrides?.selectedAuthorPubkey ?? null
        : columnRoute?.selectedAuthorPubkey ?? currentState.selectedAuthorPubkey;
      const nextSelectedDirectMessagePeerPubkey = hasOverride('selectedDirectMessagePeerPubkey')
        ? overrides?.selectedDirectMessagePeerPubkey ?? null
        : columnRoute?.selectedDirectMessagePeerPubkey ??
          currentState.selectedDirectMessagePeerPubkey;
      const nextSelectedLiveSessionId = hasOverride('selectedLiveSessionId')
        ? overrides?.selectedLiveSessionId ?? null
        : columnRoute?.selectedLiveSessionId ?? currentState.selectedLiveSessionId;
      const nextSelectedGameRoomId = hasOverride('selectedGameRoomId')
        ? overrides?.selectedGameRoomId ?? null
        : columnRoute?.selectedGameRoomId ?? currentState.selectedGameRoomId;
      const nextSettingsOpen = hasOverride('settingsOpen')
        ? overrides?.settingsOpen ?? false
        : currentState.shellChromeState.settingsOpen;
      const nextSettingsSection =
        overrides?.settingsSection ?? currentState.shellChromeState.activeSettingsSection;
      let nextSelectedChannelId = columnRoute?.selectedChannelId ?? null;

      if (hasOverride('composeTarget')) {
        nextSelectedChannelId =
          overrides?.composeTarget?.kind === 'private_channel'
            ? overrides.composeTarget.channel_id
            : null;
      } else if (hasOverride('timelineScope')) {
        nextSelectedChannelId =
          overrides?.timelineScope?.kind === 'channel'
            ? overrides.timelineScope.channel_id
            : null;
      }

      const nextUrl = buildShellUrl({
        activeTopic: nextTopic,
        focusedObjectId: nextFocusedObjectId,
        primarySection: nextPrimarySection,
        profileConnectionsView: nextProfileConnectionsView,
        profileMode: nextProfileMode,
        selectedAuthorPubkey: nextSelectedAuthorPubkey,
        selectedChannelId: nextSelectedChannelId,
        selectedDirectMessagePeerPubkey: nextSelectedDirectMessagePeerPubkey,
        selectedGameRoomId: nextSelectedGameRoomId,
        selectedLiveSessionId: nextSelectedLiveSessionId,
        selectedThread: nextSelectedThread,
        settingsOpen: nextSettingsOpen,
        settingsSection: nextSettingsSection,
        timelineView: nextTimelineView,
      });
      const currentUrl = `${resolvedRouteLocation.pathname}${resolvedRouteLocation.search}`;

      if (currentUrl !== nextUrl) {
        pendingRouteUrlRef.current = nextUrl;
        navigate(nextUrl, { replace: mode === 'replace' });
        return;
      }

      pendingRouteUrlRef.current = null;
    },
    [
      navigate,
      pendingRouteUrlRef,
      resolvedRouteLocation.pathname,
      resolvedRouteLocation.search,
      storeApi,
    ]
  );
}
