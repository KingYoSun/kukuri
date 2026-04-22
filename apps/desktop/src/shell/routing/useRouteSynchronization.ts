import { useEffect, type MutableRefObject } from 'react';
import type { NavigateFunction } from 'react-router-dom';

import type { PrimarySection } from '@/components/shell/types';
import {
  PRIMARY_SECTION_PATHS,
  isProfileConnectionsView,
  isSettingsSection,
  parseLegacyRequestedChannel,
  parsePrimarySectionPath,
  type DesktopShellRouteOverrides,
  type HashRouteLocation,
  type OpenAuthorOptions,
  type OpenThreadOptions,
} from '@/shell/routes';
import type {
  DesktopShellState,
  DesktopShellStateValue,
  DesktopShellStore,
  DesktopShellStoreApi,
} from '@/shell/store';
import {
  isHex64,
  privateComposeTarget,
  privateTimelineScope,
} from '@/shell/selectors';

type OpenDirectMessagePaneOptions = {
  historyMode?: 'push' | 'replace';
  normalizeOnError?: boolean;
  preserveAuthorPane?: boolean;
  preservedAuthorPubkey?: string | null;
};

type UseRouteSynchronizationArgs = {
  loadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>;
  lastObservedRouteUrlRef: MutableRefObject<string>;
  navigate: NavigateFunction;
  openAuthorDetail: (authorPubkey: string, options?: OpenAuthorOptions) => Promise<void>;
  openDirectMessagePane: (
    peerPubkey: string,
    options?: OpenDirectMessagePaneOptions
  ) => Promise<void>;
  openThread: (threadId: string, options?: OpenThreadOptions) => Promise<void>;
  pendingRouteUrlRef: MutableRefObject<string | null>;
  resolvedRouteLocation: HashRouteLocation;
  routeSection: PrimarySection;
  scheduleAnimationFrame: (callback: () => void) => void;
  state: DesktopShellStore;
  syncRoute: (mode?: 'push' | 'replace', overrides?: DesktopShellRouteOverrides) => void;
  storeApi: DesktopShellStoreApi;
};

export function useRouteSynchronization({
  loadTopics,
  lastObservedRouteUrlRef,
  navigate,
  openAuthorDetail,
  openDirectMessagePane,
  openThread,
  pendingRouteUrlRef,
  resolvedRouteLocation,
  routeSection,
  scheduleAnimationFrame,
  state,
  syncRoute,
  storeApi,
}: UseRouteSynchronizationArgs) {
  const {
    activeTopic,
    directMessagePaneOpen,
    focusedObjectId,
    gamePanelStateByTopic,
    gameRoomsByTopic,
    joinedChannelsByTopic,
    livePanelStateByTopic,
    liveSessionsByTopic,
    selectedAuthor,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedDirectMessagePeerPubkey,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
    shellChromeState,
    thread,
    trackedTopics,
  } = state;

  useEffect(() => {
    const setField = <K extends keyof DesktopShellState>(
      key: K,
      value: DesktopShellStateValue<K>
    ) => {
      storeApi.getState().setField(key, value);
    };
    const setActiveTopic = (value: DesktopShellStateValue<'activeTopic'>) =>
      setField('activeTopic', value);
    const setAuthorError = (value: DesktopShellStateValue<'authorError'>) =>
      setField('authorError', value);
    const setComposeChannelByTopic = (value: DesktopShellStateValue<'composeChannelByTopic'>) =>
      setField('composeChannelByTopic', value);
    const setDirectMessageError = (value: DesktopShellStateValue<'directMessageError'>) =>
      setField('directMessageError', value);
    const setDirectMessagePaneOpen = (
      value: DesktopShellStateValue<'directMessagePaneOpen'>
    ) => setField('directMessagePaneOpen', value);
    const setFocusedObjectId = (value: DesktopShellStateValue<'focusedObjectId'>) =>
      setField('focusedObjectId', value);
    const setLastNonNotificationsRoute = (
      value: DesktopShellStateValue<'lastNonNotificationsRoute'>
    ) => setField('lastNonNotificationsRoute', value);
    const setReplyTarget = (value: DesktopShellStateValue<'replyTarget'>) =>
      setField('replyTarget', value);
    const setRepostTarget = (value: DesktopShellStateValue<'repostTarget'>) =>
      setField('repostTarget', value);
    const setSelectedAuthor = (value: DesktopShellStateValue<'selectedAuthor'>) =>
      setField('selectedAuthor', value);
    const setSelectedAuthorPubkey = (
      value: DesktopShellStateValue<'selectedAuthorPubkey'>
    ) => setField('selectedAuthorPubkey', value);
    const setSelectedChannelIdByTopic = (
      value: DesktopShellStateValue<'selectedChannelIdByTopic'>
    ) => setField('selectedChannelIdByTopic', value);
    const setSelectedDirectMessagePeerPubkey = (
      value: DesktopShellStateValue<'selectedDirectMessagePeerPubkey'>
    ) => setField('selectedDirectMessagePeerPubkey', value);
    const setSelectedGameRoomId = (value: DesktopShellStateValue<'selectedGameRoomId'>) =>
      setField('selectedGameRoomId', value);
    const setSelectedLiveSessionId = (value: DesktopShellStateValue<'selectedLiveSessionId'>) =>
      setField('selectedLiveSessionId', value);
    const setSelectedThread = (value: DesktopShellStateValue<'selectedThread'>) =>
      setField('selectedThread', value);
    const setShellChromeState = (value: DesktopShellStateValue<'shellChromeState'>) =>
      setField('shellChromeState', value);
    const setThread = (value: DesktopShellStateValue<'thread'>) => setField('thread', value);
    const setTimelineScopeByTopic = (value: DesktopShellStateValue<'timelineScopeByTopic'>) =>
      setField('timelineScopeByTopic', value);

    const currentUrl = `${resolvedRouteLocation.pathname}${resolvedRouteLocation.search}`;
    const routeChanged = lastObservedRouteUrlRef.current !== currentUrl;
    if (pendingRouteUrlRef.current && pendingRouteUrlRef.current !== currentUrl) {
      if (!routeChanged) {
        return;
      }
      pendingRouteUrlRef.current = null;
    }
    pendingRouteUrlRef.current = null;
    lastObservedRouteUrlRef.current = currentUrl;
    if (routeSection !== 'notifications') {
      setLastNonNotificationsRoute(currentUrl);
    }

    if (!parsePrimarySectionPath(resolvedRouteLocation.pathname)) {
      navigate(`${PRIMARY_SECTION_PATHS.timeline}${resolvedRouteLocation.search}`, {
        replace: true,
      });
      return;
    }

    const params = new URLSearchParams(resolvedRouteLocation.search);
    const requestedTopic = params.get('topic')?.trim() ?? null;
    const requestedChannelParam = params.get('channel')?.trim() ?? null;
    const requestedTimelineView = params.get('timelineView');
    const requestedTimelineScopeValue = params.get('timelineScope');
    const requestedComposeTargetValue = params.get('composeTarget');
    const requestedSettingsSection = params.get('settings');
    const requestedContext = params.get('context');
    const requestedProfileMode = params.get('profileMode');
    const requestedConnectionsView = params.get('connectionsView');
    const requestedThreadId = params.get('threadId');
    const requestedFocusObjectId = params.get('focusObjectId')?.trim() ?? null;
    const requestedAuthorPubkey = params.get('authorPubkey');
    const requestedPeerPubkey = params.get('peerPubkey');
    const requestedSessionId = params.get('sessionId')?.trim() ?? null;
    const requestedRoomId = params.get('roomId')?.trim() ?? null;

    let nextTopic = activeTopic;
    let shouldReload = false;
    let shouldNormalize = false;
    let normalizedSelectedThread: string | null = selectedThread;
    let normalizedFocusedObjectId: string | null = focusedObjectId;
    let normalizedSelectedAuthorPubkey: string | null = selectedAuthorPubkey;
    let normalizedSelectedDirectMessagePeerPubkey: string | null =
      selectedDirectMessagePeerPubkey;
    let normalizedSelectedLiveSessionId: string | null = selectedLiveSessionId;
    let normalizedSelectedGameRoomId: string | null = selectedGameRoomId;

    if (requestedTopic) {
      if (trackedTopics.includes(requestedTopic)) {
        if (requestedTopic !== activeTopic) {
          nextTopic = requestedTopic;
          setActiveTopic(requestedTopic);
          shouldReload = true;
        }
      } else {
        shouldNormalize = true;
      }
    } else {
      shouldNormalize = true;
    }

    const nextTimelineView =
      routeSection === 'timeline' && requestedTimelineView === 'bookmarks' ? 'bookmarks' : 'feed';
    const joinedChannelsForTopic = joinedChannelsByTopic[nextTopic] ?? [];
    const liveSessionsForTopic = liveSessionsByTopic[nextTopic] ?? [];
    const gameRoomsForTopic = gameRoomsByTopic[nextTopic] ?? [];
    const livePanelState = livePanelStateByTopic[nextTopic];
    const gamePanelState = gamePanelStateByTopic[nextTopic];
    const currentSelectedChannelIdForTopic = selectedChannelIdByTopic[nextTopic] ?? null;
    const allowChannelRouteParam =
      routeSection !== 'messages' && routeSection !== 'notifications';
    let nextSelectedChannelId = currentSelectedChannelIdForTopic;
    if (allowChannelRouteParam && nextTimelineView !== 'bookmarks') {
      nextSelectedChannelId = requestedChannelParam;
      if (!nextSelectedChannelId) {
        nextSelectedChannelId = parseLegacyRequestedChannel(
          requestedTimelineScopeValue,
          requestedComposeTargetValue
        );
      }
    } else if (requestedChannelParam) {
      shouldNormalize = true;
    }
    if (requestedTimelineScopeValue || requestedComposeTargetValue) {
      shouldNormalize = true;
    }
    if (
      nextTimelineView !== 'bookmarks' &&
      nextSelectedChannelId &&
      !joinedChannelsForTopic.some((channel) => channel.channel_id === nextSelectedChannelId)
    ) {
      shouldNormalize = true;
      nextSelectedChannelId = null;
    }

    if (currentSelectedChannelIdForTopic !== nextSelectedChannelId) {
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [nextTopic]: nextSelectedChannelId,
      }));
      setTimelineScopeByTopic((current) => ({
        ...current,
        [nextTopic]: privateTimelineScope(nextSelectedChannelId),
      }));
      setComposeChannelByTopic((current) => ({
        ...current,
        [nextTopic]: privateComposeTarget(nextSelectedChannelId),
      }));
      shouldReload = true;
    }

    if (requestedContext === 'dm' && routeSection !== 'messages') {
      scheduleAnimationFrame(() => {
        syncRoute('replace', {
          activeTopic: nextTopic,
          primarySection: 'messages',
          selectedAuthorPubkey: null,
          selectedDirectMessagePeerPubkey:
            requestedPeerPubkey && isHex64(requestedPeerPubkey) ? requestedPeerPubkey : null,
          selectedThread: null,
        });
      });
      return;
    }

    const nextSettingsOpen = isSettingsSection(requestedSettingsSection);
    const nextSettingsResolvedSection = isSettingsSection(requestedSettingsSection)
      ? requestedSettingsSection
      : shellChromeState.activeSettingsSection;
    const nextProfileMode =
      routeSection === 'profile'
        ? requestedProfileMode === 'edit'
          ? 'edit'
          : requestedProfileMode === 'connections'
            ? 'connections'
            : 'overview'
        : 'overview';
    const nextProfileConnectionsView =
      routeSection === 'profile' && requestedProfileMode === 'connections'
        ? isProfileConnectionsView(requestedConnectionsView)
          ? requestedConnectionsView
          : 'following'
        : shellChromeState.profileConnectionsView;

    if (
      shellChromeState.activePrimarySection !== routeSection ||
      shellChromeState.timelineView !== nextTimelineView ||
      shellChromeState.activeSettingsSection !== nextSettingsResolvedSection ||
      shellChromeState.settingsOpen !== nextSettingsOpen ||
      shellChromeState.profileMode !== nextProfileMode ||
      shellChromeState.profileConnectionsView !== nextProfileConnectionsView
    ) {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: routeSection,
        timelineView: nextTimelineView,
        activeSettingsSection: nextSettingsResolvedSection,
        settingsOpen: nextSettingsOpen,
        profileMode: nextProfileMode,
        profileConnectionsView: nextProfileConnectionsView,
      }));
    }

    if (requestedTimelineView && requestedTimelineView !== 'bookmarks') {
      shouldNormalize = true;
    }
    if (requestedTimelineView && routeSection !== 'timeline') {
      shouldNormalize = true;
    }
    if (requestedSettingsSection && !isSettingsSection(requestedSettingsSection)) {
      shouldNormalize = true;
    }
    if (
      requestedProfileMode &&
      requestedProfileMode !== 'edit' &&
      requestedProfileMode !== 'connections'
    ) {
      shouldNormalize = true;
    }
    if (requestedProfileMode && routeSection !== 'profile') {
      shouldNormalize = true;
    }
    if (
      requestedConnectionsView &&
      (requestedProfileMode !== 'connections' ||
        !isProfileConnectionsView(requestedConnectionsView))
    ) {
      shouldNormalize = true;
    }
    if (routeSection === 'messages' && requestedContext) {
      shouldNormalize = true;
    }
    if (
      routeSection === 'notifications' &&
      (requestedTimelineView ||
        requestedChannelParam ||
        requestedContext ||
        requestedProfileMode ||
        requestedConnectionsView ||
        requestedThreadId ||
        requestedFocusObjectId ||
        requestedAuthorPubkey ||
        requestedPeerPubkey ||
        requestedSessionId ||
        requestedRoomId)
    ) {
      shouldNormalize = true;
    }

    if (nextTimelineView === 'bookmarks') {
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (requestedContext) {
        shouldNormalize = true;
      }
      if (requestedFocusObjectId || requestedSessionId || requestedRoomId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (selectedAuthorPubkey) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen) {
        setDirectMessagePaneOpen(false);
      }
      if (selectedDirectMessagePeerPubkey) {
        setSelectedDirectMessagePeerPubkey(null);
      }
      setDirectMessageError(null);
    }
    if (routeSection === 'messages') {
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      if (requestedThreadId) {
        shouldNormalize = true;
      }
      if (requestedFocusObjectId || requestedSessionId || requestedRoomId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (!directMessagePaneOpen) {
        setDirectMessagePaneOpen(true);
      }
      if (!requestedPeerPubkey) {
        normalizedSelectedDirectMessagePeerPubkey = null;
        if (selectedDirectMessagePeerPubkey) {
          setSelectedDirectMessagePeerPubkey(null);
        }
        setDirectMessageError(null);
      } else if (!isHex64(requestedPeerPubkey)) {
        shouldNormalize = true;
        normalizedSelectedDirectMessagePeerPubkey = null;
        if (selectedDirectMessagePeerPubkey) {
          setSelectedDirectMessagePeerPubkey(null);
        }
      } else if (
        requestedPeerPubkey !== selectedDirectMessagePeerPubkey ||
        !directMessagePaneOpen
      ) {
        normalizedSelectedDirectMessagePeerPubkey = requestedPeerPubkey;
        void openDirectMessagePane(requestedPeerPubkey, {
          historyMode: 'replace',
          normalizeOnError: true,
          preserveAuthorPane: requestedAuthorPubkey !== null && isHex64(requestedAuthorPubkey),
          preservedAuthorPubkey:
            requestedAuthorPubkey && isHex64(requestedAuthorPubkey)
              ? requestedAuthorPubkey
              : null,
        });
      } else {
        normalizedSelectedDirectMessagePeerPubkey = requestedPeerPubkey;
      }
      if (!requestedAuthorPubkey) {
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!isHex64(requestedAuthorPubkey)) {
        shouldNormalize = true;
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (
        requestedAuthorPubkey !== selectedAuthorPubkey ||
        !selectedAuthor ||
        (requestedPeerPubkey ?? null) !== (selectedDirectMessagePeerPubkey ?? null)
      ) {
        normalizedSelectedAuthorPubkey = requestedAuthorPubkey;
        void openAuthorDetail(requestedAuthorPubkey, {
          historyMode: 'replace',
          normalizeOnError: true,
          preserveDirectMessageContext: true,
          directMessagePeerPubkey:
            requestedPeerPubkey && isHex64(requestedPeerPubkey) ? requestedPeerPubkey : null,
        });
      } else {
        normalizedSelectedAuthorPubkey = requestedAuthorPubkey;
      }
    } else if (routeSection === 'notifications') {
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (selectedThread) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (selectedAuthorPubkey) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
    } else if (
      routeSection === 'timeline' &&
      nextTimelineView !== 'bookmarks' &&
      requestedContext === 'thread'
    ) {
      normalizedSelectedDirectMessagePeerPubkey = null;
      const threadReadyForNestedAuthor =
        requestedThreadId !== null &&
        requestedThreadId.length > 0 &&
        requestedThreadId === selectedThread &&
        thread.length > 0;

      if (!requestedThreadId) {
        shouldNormalize = true;
        normalizedSelectedThread = null;
        normalizedFocusedObjectId = null;
        normalizedSelectedAuthorPubkey = null;
        if (selectedThread || selectedAuthorPubkey) {
          setSelectedThread(null);
          setFocusedObjectId(null);
          setThread([]);
          setReplyTarget(null);
          setRepostTarget(null);
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (requestedThreadId !== selectedThread || thread.length === 0) {
        normalizedSelectedThread = requestedThreadId;
        normalizedFocusedObjectId = requestedFocusObjectId;
        void openThread(requestedThreadId, {
          focusObjectId: requestedFocusObjectId,
          historyMode: 'replace',
          normalizeOnEmpty: true,
          topic: nextTopic,
        });
      } else {
        normalizedSelectedThread = requestedThreadId;
        if (!requestedFocusObjectId) {
          normalizedFocusedObjectId = null;
          if (focusedObjectId) {
            setFocusedObjectId(null);
          }
        } else if (thread.some((item) => item.object_id === requestedFocusObjectId)) {
          normalizedFocusedObjectId = requestedFocusObjectId;
          if (focusedObjectId !== requestedFocusObjectId) {
            setFocusedObjectId(requestedFocusObjectId);
          }
        } else {
          shouldNormalize = true;
          normalizedFocusedObjectId = null;
          if (focusedObjectId) {
            setFocusedObjectId(null);
          }
        }
      }
      if (!requestedAuthorPubkey) {
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!isHex64(requestedAuthorPubkey)) {
        shouldNormalize = true;
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!threadReadyForNestedAuthor) {
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (
        requestedAuthorPubkey !== selectedAuthorPubkey ||
        !selectedAuthor ||
        requestedThreadId !== selectedThread
      ) {
        normalizedSelectedAuthorPubkey = requestedAuthorPubkey;
        void openAuthorDetail(requestedAuthorPubkey, {
          fromThread: true,
          historyMode: 'replace',
          normalizeOnError: true,
          threadId: requestedThreadId,
        });
      } else {
        normalizedSelectedAuthorPubkey = requestedAuthorPubkey;
      }
    } else if (
      routeSection === 'timeline' &&
      nextTimelineView !== 'bookmarks' &&
      requestedContext === 'author'
    ) {
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (requestedThreadId) {
        shouldNormalize = true;
      }
      if (requestedFocusObjectId || requestedSessionId || requestedRoomId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (!requestedAuthorPubkey) {
        shouldNormalize = true;
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (!isHex64(requestedAuthorPubkey)) {
        shouldNormalize = true;
        normalizedSelectedAuthorPubkey = null;
        if (selectedAuthorPubkey) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (requestedAuthorPubkey !== selectedAuthorPubkey || !selectedAuthor) {
        normalizedSelectedAuthorPubkey = requestedAuthorPubkey;
        void openAuthorDetail(requestedAuthorPubkey, {
          historyMode: 'replace',
          normalizeOnError: true,
        });
      } else {
        normalizedSelectedAuthorPubkey = requestedAuthorPubkey;
      }
    } else if (routeSection === 'live') {
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (
        requestedContext ||
        requestedThreadId ||
        requestedFocusObjectId ||
        requestedAuthorPubkey ||
        requestedPeerPubkey ||
        requestedRoomId
      ) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (selectedAuthorPubkey) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
      normalizedSelectedGameRoomId = null;
      if (selectedGameRoomId) {
        setSelectedGameRoomId(null);
      }
      if (!requestedSessionId) {
        normalizedSelectedLiveSessionId = null;
        if (selectedLiveSessionId) {
          setSelectedLiveSessionId(null);
        }
      } else if (
        livePanelState?.status === 'ready' &&
        !liveSessionsForTopic.some((session) => session.session_id === requestedSessionId)
      ) {
        shouldNormalize = true;
        normalizedSelectedLiveSessionId = null;
        if (selectedLiveSessionId) {
          setSelectedLiveSessionId(null);
        }
      } else {
        normalizedSelectedLiveSessionId = requestedSessionId;
        if (selectedLiveSessionId !== requestedSessionId) {
          setSelectedLiveSessionId(requestedSessionId);
        }
      }
    } else if (routeSection === 'game') {
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (
        requestedContext ||
        requestedThreadId ||
        requestedFocusObjectId ||
        requestedAuthorPubkey ||
        requestedPeerPubkey ||
        requestedSessionId
      ) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (selectedAuthorPubkey) {
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
      normalizedSelectedLiveSessionId = null;
      if (selectedLiveSessionId) {
        setSelectedLiveSessionId(null);
      }
      if (!requestedRoomId) {
        normalizedSelectedGameRoomId = null;
        if (selectedGameRoomId) {
          setSelectedGameRoomId(null);
        }
      } else if (
        gamePanelState?.status === 'ready' &&
        !gameRoomsForTopic.some((room) => room.room_id === requestedRoomId)
      ) {
        shouldNormalize = true;
        normalizedSelectedGameRoomId = null;
        if (selectedGameRoomId) {
          setSelectedGameRoomId(null);
        }
      } else {
        normalizedSelectedGameRoomId = requestedRoomId;
        if (selectedGameRoomId !== requestedRoomId) {
          setSelectedGameRoomId(requestedRoomId);
        }
      }
    } else if (routeSection === 'timeline' && nextTimelineView !== 'bookmarks' && requestedContext) {
      shouldNormalize = true;
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (selectedThread || selectedAuthorPubkey) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (focusedObjectId) {
        setFocusedObjectId(null);
      }
      if (directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
    } else {
      if (
        requestedThreadId ||
        requestedFocusObjectId ||
        requestedAuthorPubkey ||
        requestedPeerPubkey ||
        requestedSessionId ||
        requestedRoomId
      ) {
        shouldNormalize = true;
      }
      normalizedSelectedThread = null;
      normalizedFocusedObjectId = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (
        selectedThread ||
        focusedObjectId ||
        selectedAuthorPubkey ||
        directMessagePaneOpen ||
        selectedDirectMessagePeerPubkey
      ) {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
    }

    if (shouldReload) {
      void loadTopics(
        trackedTopics,
        nextTopic,
        requestedContext === 'thread' ? requestedThreadId : null
      ).catch(() => undefined);
    }

    if (shouldNormalize) {
      scheduleAnimationFrame(() => {
        syncRoute('replace', {
          activeTopic: nextTopic,
          composeTarget: privateComposeTarget(nextSelectedChannelId),
          focusedObjectId: normalizedFocusedObjectId,
          primarySection: routeSection,
          profileConnectionsView: nextProfileConnectionsView,
          profileMode: nextProfileMode,
          selectedAuthorPubkey: normalizedSelectedAuthorPubkey,
          selectedDirectMessagePeerPubkey: normalizedSelectedDirectMessagePeerPubkey,
          selectedGameRoomId: normalizedSelectedGameRoomId,
          selectedLiveSessionId: normalizedSelectedLiveSessionId,
          selectedThread: normalizedSelectedThread,
          settingsOpen: nextSettingsOpen,
          settingsSection: nextSettingsResolvedSection,
          timelineScope: privateTimelineScope(nextSelectedChannelId),
          timelineView: nextTimelineView,
        });
      });
    }
  }, [
    activeTopic,
    directMessagePaneOpen,
    focusedObjectId,
    gamePanelStateByTopic,
    gameRoomsByTopic,
    joinedChannelsByTopic,
    loadTopics,
    livePanelStateByTopic,
    liveSessionsByTopic,
    lastObservedRouteUrlRef,
    navigate,
    openAuthorDetail,
    openDirectMessagePane,
    openThread,
    pendingRouteUrlRef,
    resolvedRouteLocation.pathname,
    resolvedRouteLocation.search,
    routeSection,
    scheduleAnimationFrame,
    selectedAuthor,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedDirectMessagePeerPubkey,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
    shellChromeState.activePrimarySection,
    shellChromeState.activeSettingsSection,
    shellChromeState.profileConnectionsView,
    shellChromeState.profileMode,
    shellChromeState.settingsOpen,
    shellChromeState.timelineView,
    storeApi,
    syncRoute,
    thread,
    trackedTopics,
  ]);
}
