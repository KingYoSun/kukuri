import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type MutableRefObject,
  type RefObject,
} from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

import type { DesktopApi } from '@/lib/api';

import type {
  PrimarySection,
  ProfileConnectionsView,
  TimelineWorkspaceView,
} from '@/components/shell/types';
import {
  type OpenAuthorOptions,
  type OpenThreadOptions,
  parsePrimarySectionPath,
  resolveHashBackedRouteLocation,
} from '@/shell/routes';
import { THREAD_TIMELINE_LIMIT } from '@/shell/pagination';
import { useRouteSynchronization } from '@/shell/routing/useRouteSynchronization';
import { useSyncRoute } from '@/shell/routing/useSyncRoute';
import {
  authorViewFromDirectMessageConversation,
  mergeKnownAuthors,
  messageFromError,
} from '@/shell/selectors';
import {
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  useDesktopShellStoreApi,
} from '@/shell/store';

type UseDesktopShellRoutingArgs = {
  api: DesktopApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
  loadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>;
  primarySectionRefs: MutableRefObject<Partial<Record<PrimarySection, HTMLElement | null>>>;
  navTriggerRef: RefObject<HTMLButtonElement | null>;
  settingsTriggerRef: RefObject<HTMLButtonElement | null>;
  pendingRouteUrlRef: MutableRefObject<string | null>;
  didSyncRouteSectionRef: MutableRefObject<boolean>;
};

export function useDesktopShellRouting({
  api,
  translate,
  loadTopics,
  primarySectionRefs,
  navTriggerRef,
  settingsTriggerRef,
  pendingRouteUrlRef,
  didSyncRouteSectionRef,
}: UseDesktopShellRoutingArgs) {
  const location = useLocation();
  const navigate = useNavigate();
  const storeApi = useDesktopShellStoreApi();
  const state = useDesktopShellStore();
  const {
    activeTopic,
    selectedThread,
    selectedAuthorPubkey,
    selectedDirectMessagePeerPubkey,
    lastNonNotificationsRoute,
    shellChromeState,
  } = state;

  const setActiveTopic = useDesktopShellFieldSetter('activeTopic');
  const setSelectedThread = useDesktopShellFieldSetter('selectedThread');
  const setFocusedObjectId = useDesktopShellFieldSetter('focusedObjectId');
  const setThread = useDesktopShellFieldSetter('thread');
  const setThreadNextCursorById = useDesktopShellFieldSetter('threadNextCursorById');
  const setReplyTarget = useDesktopShellFieldSetter('replyTarget');
  const setRepostTarget = useDesktopShellFieldSetter('repostTarget');
  const setSelectedAuthorPubkey = useDesktopShellFieldSetter('selectedAuthorPubkey');
  const setSelectedAuthor = useDesktopShellFieldSetter('selectedAuthor');
  const setSelectedAuthorTimeline = useDesktopShellFieldSetter('selectedAuthorTimeline');
  const setAuthorError = useDesktopShellFieldSetter('authorError');
  const setDirectMessagePaneOpen = useDesktopShellFieldSetter('directMessagePaneOpen');
  const setSelectedDirectMessagePeerPubkey = useDesktopShellFieldSetter(
    'selectedDirectMessagePeerPubkey'
  );
  const setDirectMessages = useDesktopShellFieldSetter('directMessages');
  const setDirectMessageTimelineByPeer = useDesktopShellFieldSetter('directMessageTimelineByPeer');
  const setDirectMessageStatusByPeer = useDesktopShellFieldSetter('directMessageStatusByPeer');
  const setDirectMessageError = useDesktopShellFieldSetter('directMessageError');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const setSelectedLiveSessionId = useDesktopShellFieldSetter('selectedLiveSessionId');
  const setSelectedGameRoomId = useDesktopShellFieldSetter('selectedGameRoomId');
  const setError = useDesktopShellFieldSetter('error');
  const setLastNonNotificationsRoute = useDesktopShellFieldSetter('lastNonNotificationsRoute');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');
  const resolvedRouteLocation = useMemo(
    () => resolveHashBackedRouteLocation(location.pathname, location.search),
    [location.pathname, location.search]
  );

  const routeSection = useMemo(
    () =>
      parsePrimarySectionPath(resolvedRouteLocation.pathname) ??
      shellChromeState.activePrimarySection,
    [resolvedRouteLocation.pathname, shellChromeState.activePrimarySection]
  );
  const pendingAnimationFrameIdsRef = useRef<number[]>([]);
  const lastObservedRouteUrlRef = useRef(
    `${resolvedRouteLocation.pathname}${resolvedRouteLocation.search}`
  );

  const scheduleAnimationFrame = useCallback((callback: () => void) => {
    const frameId = window.requestAnimationFrame(() => {
      pendingAnimationFrameIdsRef.current = pendingAnimationFrameIdsRef.current.filter(
        (candidate) => candidate !== frameId
      );
      callback();
    });
    pendingAnimationFrameIdsRef.current.push(frameId);
  }, []);

  useEffect(() => {
    return () => {
      for (const frameId of pendingAnimationFrameIdsRef.current) {
        window.cancelAnimationFrame(frameId);
      }
      pendingAnimationFrameIdsRef.current = [];
    };
  }, []);

  const syncRoute = useSyncRoute({
    navigate,
    pendingRouteUrlRef,
    resolvedRouteLocation,
    storeApi,
  });

  const setNavOpen = useCallback(
    (open: boolean, restoreToTrigger = false) => {
      setShellChromeState((current) => ({
        ...current,
        navOpen: open,
      }));
      if (!open && restoreToTrigger) {
        scheduleAnimationFrame(() => {
          navTriggerRef.current?.focus();
        });
      }
    },
    [navTriggerRef, scheduleAnimationFrame, setShellChromeState]
  );

  const setSettingsOpen = useCallback(
    (open: boolean, restoreToTrigger = false) => {
      setShellChromeState((current) => ({
        ...current,
        settingsOpen: open,
      }));
      if (!open && restoreToTrigger) {
        scheduleAnimationFrame(() => {
          settingsTriggerRef.current?.focus();
        });
      }
      syncRoute(open ? 'push' : 'replace', {
        settingsOpen: open,
      });
    },
    [scheduleAnimationFrame, setShellChromeState, settingsTriggerRef, syncRoute]
  );

  const setPrimarySectionRef = useCallback(
    (section: PrimarySection) => {
      return (element: HTMLElement | null) => {
        primarySectionRefs.current[section] = element;
      };
    },
    [primarySectionRefs]
  );

  const openDirectMessagePane = useCallback(
    async (
      peerPubkey: string,
      options?: {
        historyMode?: 'push' | 'replace';
        normalizeOnError?: boolean;
        preserveAuthorPane?: boolean;
        preservedAuthorPubkey?: string | null;
      }
    ) => {
      try {
        const [conversation, timeline, status] = await Promise.all([
          api.openDirectMessage(peerPubkey),
          api.listDirectMessageMessages(peerPubkey, null, 100),
          api.getDirectMessageStatus(peerPubkey),
        ]);
        const preserveSelectedAuthor =
          options?.preserveAuthorPane ??
          (selectedDirectMessagePeerPubkey === peerPubkey && selectedAuthorPubkey !== null);
        const nextSelectedAuthorPubkey = preserveSelectedAuthor
          ? options?.preservedAuthorPubkey ?? selectedAuthorPubkey
          : null;
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        if (!preserveSelectedAuthor) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setSelectedAuthorTimeline([]);
          setAuthorError(null);
        }
        setDirectMessages((current) => {
          const remaining = current.filter((entry) => entry.peer_pubkey !== conversation.peer_pubkey);
          return [conversation, ...remaining];
        });
        setDirectMessageTimelineByPeer((current) => ({
          ...current,
          [peerPubkey]: timeline.items,
        }));
        setDirectMessageStatusByPeer((current) => ({
          ...current,
          [peerPubkey]: status,
        }));
        setKnownAuthorsByPubkey((current) =>
          mergeKnownAuthors(current, [authorViewFromDirectMessageConversation(conversation)])
        );
        setShellChromeState((current) => ({
          ...current,
          activePrimarySection: 'messages',
          navOpen: false,
        }));
        setDirectMessagePaneOpen(true);
        setSelectedLiveSessionId(null);
        setSelectedGameRoomId(null);
        setSelectedDirectMessagePeerPubkey(peerPubkey);
        setDirectMessageError(null);
        syncRoute(options?.historyMode ?? 'push', {
          focusedObjectId: null,
          primarySection: 'messages',
          selectedGameRoomId: null,
          selectedAuthorPubkey: nextSelectedAuthorPubkey,
          selectedDirectMessagePeerPubkey: peerPubkey,
          selectedLiveSessionId: null,
          selectedThread: null,
        });
      } catch (openError) {
        const nextError = messageFromError(openError, 'failed to open direct message');
        setDirectMessageError(nextError);
        if (options?.normalizeOnError) {
          setDirectMessagePaneOpen(true);
          setSelectedDirectMessagePeerPubkey(null);
          syncRoute('replace', {
            primarySection: 'messages',
            selectedDirectMessagePeerPubkey: null,
          });
        }
      }
    },
    [
      api,
      selectedAuthorPubkey,
      selectedDirectMessagePeerPubkey,
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
      setDirectMessages,
      setDirectMessageStatusByPeer,
      setDirectMessageTimelineByPeer,
      setKnownAuthorsByPubkey,
      setReplyTarget,
      setRepostTarget,
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedAuthorTimeline,
      setSelectedDirectMessagePeerPubkey,
      setSelectedGameRoomId,
      setSelectedLiveSessionId,
      setSelectedThread,
      setFocusedObjectId,
      setShellChromeState,
      setThread,
      syncRoute,
    ]
  );

  const openThread = useCallback(
    async (threadId: string, options?: OpenThreadOptions) => {
      const topic = options?.topic ?? activeTopic;
      try {
        const threadView = await api.listThread(topic, threadId, null, THREAD_TIMELINE_LIMIT);
        const nextFocusedObjectId =
          options?.focusObjectId &&
          threadView.items.some((item) => item.object_id === options.focusObjectId)
            ? options.focusObjectId
            : null;
        if (options?.normalizeOnEmpty && threadView.items.length === 0) {
          startTransition(() => {
            setSelectedThread(null);
            setFocusedObjectId(null);
            setThread([]);
            setSelectedAuthorPubkey(null);
            setSelectedAuthor(null);
            setAuthorError(null);
            setDirectMessagePaneOpen(false);
            setSelectedDirectMessagePeerPubkey(null);
            setDirectMessageError(null);
            setSelectedLiveSessionId(null);
            setSelectedGameRoomId(null);
          });
          syncRoute('replace', {
            activeTopic: topic,
            primarySection: 'timeline',
            timelineView: 'feed',
            directMessagePaneOpen: false,
            focusedObjectId: null,
            selectedGameRoomId: null,
            selectedAuthorPubkey: null,
            selectedLiveSessionId: null,
            selectedThread: null,
          });
          return;
        }
        startTransition(() => {
          setActiveTopic(topic);
          setShellChromeState((current) => ({
            ...current,
            activePrimarySection: 'timeline',
            timelineView: 'feed',
            navOpen: false,
          }));
          setSelectedThread(threadId);
          setFocusedObjectId(nextFocusedObjectId);
          setThread(threadView.items);
          setThreadNextCursorById((current) => ({
            ...current,
            [threadId]: threadView.next_cursor ?? null,
          }));
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
          setDirectMessagePaneOpen(false);
          setSelectedDirectMessagePeerPubkey(null);
          setDirectMessageError(null);
          setSelectedLiveSessionId(null);
          setSelectedGameRoomId(null);
          setError(null);
        });
        syncRoute(options?.historyMode ?? 'push', {
          activeTopic: topic,
          primarySection: 'timeline',
          timelineView: 'feed',
          directMessagePaneOpen: false,
          focusedObjectId: nextFocusedObjectId,
          selectedGameRoomId: null,
          selectedAuthorPubkey: null,
          selectedLiveSessionId: null,
          selectedThread: threadId,
        });
      } catch (threadError) {
        const nextError =
          threadError instanceof Error
            ? threadError.message
            : translate('common:errors.failedToLoadThread');
        setError(nextError);
        if (options?.normalizeOnEmpty) {
          startTransition(() => {
            setSelectedThread(null);
            setFocusedObjectId(null);
            setThread([]);
            setSelectedAuthorPubkey(null);
            setSelectedAuthor(null);
            setAuthorError(null);
            setDirectMessagePaneOpen(false);
            setSelectedDirectMessagePeerPubkey(null);
            setDirectMessageError(null);
            setSelectedLiveSessionId(null);
            setSelectedGameRoomId(null);
          });
          syncRoute('replace', {
            activeTopic: topic,
            primarySection: 'timeline',
            timelineView: 'feed',
            directMessagePaneOpen: false,
            focusedObjectId: null,
            selectedGameRoomId: null,
            selectedAuthorPubkey: null,
            selectedLiveSessionId: null,
            selectedThread: null,
          });
        }
      }
    },
    [
      activeTopic,
      api,
      setActiveTopic,
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
      setError,
      setFocusedObjectId,
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedDirectMessagePeerPubkey,
      setSelectedGameRoomId,
      setSelectedLiveSessionId,
      setSelectedThread,
      setShellChromeState,
      setThread,
      setThreadNextCursorById,
      syncRoute,
      translate,
    ]
  );

  const openAuthorDetail = useCallback(
    async (authorPubkey: string, options?: OpenAuthorOptions) => {
      try {
        const socialView = await api.getAuthorSocialView(authorPubkey);
        const nextThreadId = options?.fromThread ? (options.threadId ?? selectedThread) : null;
        const nextDirectMessagePeerPubkey = options?.preserveDirectMessageContext
          ? options.directMessagePeerPubkey ?? selectedDirectMessagePeerPubkey ?? null
          : null;
        setSelectedAuthorPubkey(authorPubkey);
        setSelectedAuthor(socialView);
        setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [socialView]));
        setSelectedAuthorTimeline([]);
        setAuthorError(null);
        if (options?.preserveDirectMessageContext) {
          setDirectMessagePaneOpen(true);
          setSelectedDirectMessagePeerPubkey(nextDirectMessagePeerPubkey);
          setDirectMessageError(null);
        } else {
          setDirectMessagePaneOpen(false);
          setSelectedDirectMessagePeerPubkey(null);
          setDirectMessageError(null);
        }
        if (!options?.fromThread) {
          setSelectedThread(null);
          setFocusedObjectId(null);
          setThread([]);
        }
        syncRoute(options?.historyMode ?? 'push', {
          primarySection: options?.preserveDirectMessageContext ? 'messages' : 'timeline',
          timelineView: options?.preserveDirectMessageContext ? undefined : 'feed',
          focusedObjectId: options?.fromThread ? undefined : null,
          selectedThread: nextThreadId,
          selectedAuthorPubkey: authorPubkey,
          selectedDirectMessagePeerPubkey: options?.preserveDirectMessageContext
            ? nextDirectMessagePeerPubkey
            : undefined,
        });
      } catch (detailError) {
        const nextError =
          detailError instanceof Error
            ? detailError.message
            : translate('common:errors.failedToLoadAuthor');
        setAuthorError(nextError);
        if (options?.normalizeOnError) {
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setSelectedAuthorTimeline([]);
          if (!options?.fromThread) {
            setSelectedThread(null);
            setFocusedObjectId(null);
            setThread([]);
          }
          syncRoute('replace', {
            primarySection: options?.preserveDirectMessageContext ? 'messages' : 'timeline',
            timelineView: options?.preserveDirectMessageContext ? undefined : 'feed',
            focusedObjectId: options?.fromThread ? undefined : null,
            selectedThread: options?.fromThread ? (options.threadId ?? selectedThread) : null,
            selectedAuthorPubkey: null,
            selectedDirectMessagePeerPubkey: options?.preserveDirectMessageContext
              ? options.directMessagePeerPubkey ?? selectedDirectMessagePeerPubkey ?? null
              : undefined,
          });
        }
      }
    },
    [
      api,
      selectedDirectMessagePeerPubkey,
      selectedThread,
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
      setKnownAuthorsByPubkey,
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedAuthorTimeline,
      setSelectedDirectMessagePeerPubkey,
      setFocusedObjectId,
      setSelectedThread,
      setThread,
      syncRoute,
      translate,
    ]
  );

  const focusPrimarySection = useCallback(
    (section: PrimarySection) => {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: section,
        profileMode: section === 'profile' ? 'overview' : current.profileMode,
        profileConnectionsView: section === 'profile' ? 'following' : current.profileConnectionsView,
        navOpen: false,
      }));
      setSelectedThread(null);
      setFocusedObjectId(null);
      setThread([]);
      setSelectedAuthorPubkey(null);
      setSelectedAuthor(null);
      setAuthorError(null);
      setDirectMessagePaneOpen(section === 'messages');
      setSelectedDirectMessagePeerPubkey(null);
      setDirectMessageError(null);
      scheduleAnimationFrame(() => {
        primarySectionRefs.current[section]?.focus();
      });
      syncRoute('push', {
        primarySection: section,
        focusedObjectId: null,
        profileMode: section === 'profile' ? 'overview' : undefined,
        profileConnectionsView: section === 'profile' ? 'following' : undefined,
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedThread: null,
      });
    },
    [
      primarySectionRefs,
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
      setFocusedObjectId,
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedDirectMessagePeerPubkey,
      setSelectedThread,
      setShellChromeState,
      setThread,
      scheduleAnimationFrame,
      syncRoute,
    ]
  );

  const toggleNotificationsSection = useCallback(() => {
    const currentUrl = `${resolvedRouteLocation.pathname}${resolvedRouteLocation.search}`;
    if (routeSection === 'notifications') {
      if (lastNonNotificationsRoute) {
        pendingRouteUrlRef.current = lastNonNotificationsRoute;
        navigate(lastNonNotificationsRoute, { replace: false });
        return;
      }
      focusPrimarySection('timeline');
      return;
    }
    setLastNonNotificationsRoute(currentUrl);
    focusPrimarySection('notifications');
  }, [
    focusPrimarySection,
    lastNonNotificationsRoute,
    navigate,
    pendingRouteUrlRef,
    resolvedRouteLocation.pathname,
    resolvedRouteLocation.search,
    routeSection,
    setLastNonNotificationsRoute,
  ]);

  const focusTimelineView = useCallback(
    (view: TimelineWorkspaceView) => {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
        timelineView: view,
        navOpen: false,
      }));
      if (view === 'bookmarks') {
        setSelectedThread(null);
        setFocusedObjectId(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setSelectedAuthorTimeline([]);
        setAuthorError(null);
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
      scheduleAnimationFrame(() => {
        primarySectionRefs.current.timeline?.focus();
      });
      syncRoute('push', {
        primarySection: 'timeline',
        timelineView: view,
        focusedObjectId: view === 'bookmarks' ? null : undefined,
        selectedAuthorPubkey: view === 'bookmarks' ? null : undefined,
        selectedThread: view === 'bookmarks' ? null : undefined,
        selectedDirectMessagePeerPubkey: view === 'bookmarks' ? null : undefined,
      });
    },
    [
      primarySectionRefs,
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
      setFocusedObjectId,
      setReplyTarget,
      setRepostTarget,
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedAuthorTimeline,
      setSelectedDirectMessagePeerPubkey,
      setSelectedThread,
      setShellChromeState,
      setThread,
      scheduleAnimationFrame,
      syncRoute,
    ]
  );

  const closeAuthorPane = useCallback(() => {
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
    syncRoute('replace', {
      selectedAuthorPubkey: null,
    });
  }, [setAuthorError, setSelectedAuthor, setSelectedAuthorTimeline, setSelectedAuthorPubkey, syncRoute]);

  const closeThreadPane = useCallback(() => {
    setSelectedThread(null);
    setFocusedObjectId(null);
    setThread([]);
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
    syncRoute('replace', {
      focusedObjectId: null,
      selectedThread: null,
      selectedAuthorPubkey: null,
    });
  }, [
    setAuthorError,
    setFocusedObjectId,
    setReplyTarget,
    setRepostTarget,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedAuthorTimeline,
    setSelectedThread,
    setThread,
    syncRoute,
  ]);

  const openDirectMessageList = useCallback(
    (historyMode: 'push' | 'replace' = 'push') => {
      setReplyTarget(null);
      setRepostTarget(null);
      setSelectedThread(null);
      setFocusedObjectId(null);
      setThread([]);
      setSelectedAuthorPubkey(null);
      setSelectedAuthor(null);
      setSelectedAuthorTimeline([]);
      setAuthorError(null);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'messages',
        navOpen: false,
      }));
      setDirectMessagePaneOpen(true);
      setSelectedDirectMessagePeerPubkey(null);
      setDirectMessageError(null);
      syncRoute(historyMode, {
        primarySection: 'messages',
        focusedObjectId: null,
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedThread: null,
      });
    },
    [
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
      setFocusedObjectId,
      setReplyTarget,
      setRepostTarget,
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedAuthorTimeline,
      setSelectedDirectMessagePeerPubkey,
      setSelectedThread,
      setShellChromeState,
      setThread,
      syncRoute,
    ]
  );

  const openProfileOverview = useCallback(() => {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'profile',
      profileMode: 'overview',
    }));
    syncRoute('push', {
      primarySection: 'profile',
      profileMode: 'overview',
    });
  }, [setShellChromeState, syncRoute]);

  const openProfileEditor = useCallback(() => {
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'profile',
      profileMode: 'edit',
    }));
    syncRoute('push', {
      primarySection: 'profile',
      profileMode: 'edit',
    });
  }, [setShellChromeState, syncRoute]);

  const openProfileConnections = useCallback(
    (view: ProfileConnectionsView = 'following') => {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'profile',
        profileMode: 'connections',
        profileConnectionsView: view,
      }));
      syncRoute('push', {
        primarySection: 'profile',
        profileMode: 'connections',
        profileConnectionsView: view,
      });
    },
    [setShellChromeState, syncRoute]
  );

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') {
        return;
      }
      if (shellChromeState.settingsOpen) {
        event.preventDefault();
        setSettingsOpen(false, true);
        return;
      }
      if (selectedAuthorPubkey) {
        event.preventDefault();
        closeAuthorPane();
        return;
      }
      if (selectedThread) {
        event.preventDefault();
        closeThreadPane();
        return;
      }
      if (shellChromeState.navOpen) {
        event.preventDefault();
        setNavOpen(false, true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [
    closeAuthorPane,
    closeThreadPane,
    selectedAuthorPubkey,
    selectedThread,
    setNavOpen,
    setSettingsOpen,
    shellChromeState.navOpen,
    shellChromeState.settingsOpen,
  ]);

  useEffect(() => {
    const shouldFocusSection = didSyncRouteSectionRef.current;
    didSyncRouteSectionRef.current = true;
    setShellChromeState((current) =>
      current.activePrimarySection === routeSection
        ? current
        : {
            ...current,
            activePrimarySection: routeSection,
          }
    );
    if (!shouldFocusSection) {
      return;
    }
    scheduleAnimationFrame(() => {
      primarySectionRefs.current[routeSection]?.focus();
    });
  }, [
    didSyncRouteSectionRef,
    primarySectionRefs,
    routeSection,
    scheduleAnimationFrame,
    setShellChromeState,
  ]);

  useRouteSynchronization({
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
  });

  return {
    routeSection,
    syncRoute,
    setNavOpen,
    setSettingsOpen,
    setPrimarySectionRef,
    focusPrimarySection,
    toggleNotificationsSection,
    focusTimelineView,
    closeAuthorPane,
    closeThreadPane,
    openDirectMessageList,
    openDirectMessagePane,
    openThread,
    openAuthorDetail,
    openProfileOverview,
    openProfileEditor,
    openProfileConnections,
  };
}
