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

import type {
  DesktopApi,
} from '@/lib/api';

import type {
  PrimarySection,
  ProfileConnectionsView,
  SettingsSection,
  TimelineWorkspaceView,
} from '@/components/shell/types';
import {
  PRIMARY_SECTION_PATHS,
  type DesktopShellRouteOverrides,
  type OpenAuthorOptions,
  type OpenThreadOptions,
  isProfileConnectionsView,
  isSettingsSection,
  parsePrimarySectionPath,
} from '@/shell/routes';
import {
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  authorViewFromDirectMessageConversation,
  isHex64,
  mergeKnownAuthors,
  messageFromError,
  privateComposeTarget,
  privateTimelineScope,
} from '@/shell/selectors';

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
  const state = useDesktopShellStore();
  const {
    activeTopic,
    trackedTopics,
    joinedChannelsByTopic,
    selectedChannelIdByTopic,
    selectedThread,
    thread,
    selectedAuthorPubkey,
    selectedAuthor,
    directMessagePaneOpen,
    selectedDirectMessagePeerPubkey,
    shellChromeState,
  } = state;

  const setActiveTopic = useDesktopShellFieldSetter('activeTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setSelectedChannelIdByTopic = useDesktopShellFieldSetter('selectedChannelIdByTopic');
  const setSelectedThread = useDesktopShellFieldSetter('selectedThread');
  const setThread = useDesktopShellFieldSetter('thread');
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
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setError = useDesktopShellFieldSetter('error');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');

  const routeSection = useMemo(
    () => parsePrimarySectionPath(location.pathname) ?? shellChromeState.activePrimarySection,
    [location.pathname, shellChromeState.activePrimarySection]
  );
  const pendingAnimationFrameIdsRef = useRef<number[]>([]);

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

  const syncRoute = useCallback(
    (mode: 'push' | 'replace' = 'replace', overrides?: DesktopShellRouteOverrides) => {
      const hasOverride = <K extends keyof DesktopShellRouteOverrides>(key: K) =>
        overrides ? Object.prototype.hasOwnProperty.call(overrides, key) : false;
      const search = new URLSearchParams();
      const nextTopic = overrides?.activeTopic ?? activeTopic;
      const nextPrimarySection = overrides?.primarySection ?? shellChromeState.activePrimarySection;
      const nextTimelineView = overrides?.timelineView ?? shellChromeState.timelineView;
      const nextProfileMode = overrides?.profileMode ?? shellChromeState.profileMode;
      const nextProfileConnectionsView =
        overrides?.profileConnectionsView ?? shellChromeState.profileConnectionsView;
      const nextSelectedThread = hasOverride('selectedThread')
        ? overrides?.selectedThread ?? null
        : selectedThread;
      const nextSelectedAuthorPubkey = hasOverride('selectedAuthorPubkey')
        ? overrides?.selectedAuthorPubkey ?? null
        : selectedAuthorPubkey;
      const nextSelectedDirectMessagePeerPubkey = hasOverride('selectedDirectMessagePeerPubkey')
        ? overrides?.selectedDirectMessagePeerPubkey ?? null
        : selectedDirectMessagePeerPubkey;
      const nextSettingsOpen = hasOverride('settingsOpen')
        ? overrides?.settingsOpen ?? false
        : shellChromeState.settingsOpen;
      const nextSettingsSection =
        overrides?.settingsSection ?? shellChromeState.activeSettingsSection;
      let nextSelectedChannelId = selectedChannelIdByTopic[nextTopic] ?? null;

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

      search.set('topic', nextTopic);
      if (
        nextPrimarySection !== 'messages' &&
        nextSelectedChannelId &&
        !(nextPrimarySection === 'timeline' && nextTimelineView === 'bookmarks')
      ) {
        search.set('channel', nextSelectedChannelId);
      }
      if (nextPrimarySection === 'timeline' && nextTimelineView === 'bookmarks') {
        search.set('timelineView', 'bookmarks');
      }
      if (nextPrimarySection === 'messages') {
        if (nextSelectedDirectMessagePeerPubkey) {
          search.set('peerPubkey', nextSelectedDirectMessagePeerPubkey);
        }
        if (nextSelectedAuthorPubkey) {
          search.set('authorPubkey', nextSelectedAuthorPubkey);
        }
      } else if (nextSelectedThread) {
        search.set('context', 'thread');
        search.set('threadId', nextSelectedThread);
        if (nextSelectedAuthorPubkey) {
          search.set('authorPubkey', nextSelectedAuthorPubkey);
        }
      } else if (nextSelectedAuthorPubkey) {
        search.set('context', 'author');
        search.set('authorPubkey', nextSelectedAuthorPubkey);
      }
      if (nextPrimarySection === 'profile' && nextProfileMode === 'edit') {
        search.set('profileMode', 'edit');
      }
      if (nextPrimarySection === 'profile' && nextProfileMode === 'connections') {
        search.set('profileMode', 'connections');
        search.set('connectionsView', nextProfileConnectionsView);
      }
      if (nextSettingsOpen) {
        search.set('settings', nextSettingsSection);
      }

      const nextPath = PRIMARY_SECTION_PATHS[nextPrimarySection];
      const nextSearch = search.toString();
      const nextUrl = nextSearch ? `${nextPath}?${nextSearch}` : nextPath;
      const currentUrl = `${location.pathname}${location.search}`;
      if (currentUrl !== nextUrl) {
        pendingRouteUrlRef.current = nextUrl;
        navigate(nextUrl, { replace: mode === 'replace' });
        return;
      }
      pendingRouteUrlRef.current = null;
    },
    [
      activeTopic,
      location.pathname,
      location.search,
      navigate,
      pendingRouteUrlRef,
      selectedAuthorPubkey,
      selectedChannelIdByTopic,
      selectedDirectMessagePeerPubkey,
      selectedThread,
      shellChromeState.activePrimarySection,
      shellChromeState.activeSettingsSection,
      shellChromeState.profileConnectionsView,
      shellChromeState.profileMode,
      shellChromeState.settingsOpen,
      shellChromeState.timelineView,
    ]
  );

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
        setSelectedDirectMessagePeerPubkey(peerPubkey);
        setDirectMessageError(null);
        syncRoute(options?.historyMode ?? 'push', {
          primarySection: 'messages',
          selectedAuthorPubkey: nextSelectedAuthorPubkey,
          selectedDirectMessagePeerPubkey: peerPubkey,
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
      setSelectedThread,
      setShellChromeState,
      setThread,
      syncRoute,
    ]
  );

  const openThread = useCallback(
    async (threadId: string, options?: OpenThreadOptions) => {
      const topic = options?.topic ?? activeTopic;
      try {
        const threadView = await api.listThread(topic, threadId, null, 50);
        if (options?.normalizeOnEmpty && threadView.items.length === 0) {
          startTransition(() => {
            setSelectedThread(null);
            setThread([]);
            setSelectedAuthorPubkey(null);
            setSelectedAuthor(null);
            setAuthorError(null);
            setDirectMessagePaneOpen(false);
            setSelectedDirectMessagePeerPubkey(null);
            setDirectMessageError(null);
          });
          syncRoute('replace', {
            activeTopic: topic,
            directMessagePaneOpen: false,
            selectedAuthorPubkey: null,
            selectedThread: null,
          });
          return;
        }
        startTransition(() => {
          setActiveTopic(topic);
          setSelectedThread(threadId);
          setThread(threadView.items);
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
          setDirectMessagePaneOpen(false);
          setSelectedDirectMessagePeerPubkey(null);
          setDirectMessageError(null);
          setError(null);
        });
        syncRoute(options?.historyMode ?? 'push', {
          activeTopic: topic,
          directMessagePaneOpen: false,
          selectedAuthorPubkey: null,
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
            setThread([]);
            setSelectedAuthorPubkey(null);
            setSelectedAuthor(null);
            setAuthorError(null);
            setDirectMessagePaneOpen(false);
            setSelectedDirectMessagePeerPubkey(null);
            setDirectMessageError(null);
          });
          syncRoute('replace', {
            activeTopic: topic,
            directMessagePaneOpen: false,
            selectedAuthorPubkey: null,
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
      setSelectedAuthor,
      setSelectedAuthorPubkey,
      setSelectedDirectMessagePeerPubkey,
      setSelectedThread,
      setThread,
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
          setThread([]);
        }
        syncRoute(options?.historyMode ?? 'push', {
          primarySection: options?.preserveDirectMessageContext ? 'messages' : undefined,
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
            setThread([]);
          }
          syncRoute('replace', {
            primarySection: options?.preserveDirectMessageContext ? 'messages' : undefined,
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
    setThread([]);
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
    syncRoute('replace', {
      selectedThread: null,
      selectedAuthorPubkey: null,
    });
  }, [
    setAuthorError,
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
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedThread: null,
      });
    },
    [
      setAuthorError,
      setDirectMessageError,
      setDirectMessagePaneOpen,
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

  useEffect(() => {
    const currentUrl = `${location.pathname}${location.search}`;
    if (pendingRouteUrlRef.current && pendingRouteUrlRef.current !== currentUrl) {
      return;
    }
    pendingRouteUrlRef.current = null;

    if (!parsePrimarySectionPath(location.pathname)) {
      navigate(`${PRIMARY_SECTION_PATHS.timeline}${location.search}`, { replace: true });
      return;
    }

    const params = new URLSearchParams(location.search);
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
    const requestedAuthorPubkey = params.get('authorPubkey');
    const requestedPeerPubkey = params.get('peerPubkey');

    let nextTopic = activeTopic;
    let shouldReload = false;
    let shouldNormalize = false;
    let normalizedSelectedThread: string | null = selectedThread;
    let normalizedSelectedAuthorPubkey: string | null = selectedAuthorPubkey;
    let normalizedSelectedDirectMessagePeerPubkey: string | null =
      selectedDirectMessagePeerPubkey;

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
    const currentSelectedChannelIdForTopic = selectedChannelIdByTopic[nextTopic] ?? null;
    let nextSelectedChannelId = currentSelectedChannelIdForTopic;
    if (nextTimelineView !== 'bookmarks') {
      nextSelectedChannelId = requestedChannelParam;
      if (!nextSelectedChannelId) {
        const legacyRequestedChannel = [requestedComposeTargetValue, requestedTimelineScopeValue]
          .filter((value): value is string => Boolean(value))
          .map((value) => {
            if (value.startsWith('channel:')) {
              return value.slice('channel:'.length);
            }
            return null;
          })
          .find((value): value is string => value !== null);
        if (legacyRequestedChannel) {
          nextSelectedChannelId = legacyRequestedChannel;
        }
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
    const nextSettingsSection = isSettingsSection(requestedSettingsSection)
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
      shellChromeState.activeSettingsSection !== nextSettingsSection ||
      shellChromeState.settingsOpen !== nextSettingsOpen ||
      shellChromeState.profileMode !== nextProfileMode ||
      shellChromeState.profileConnectionsView !== nextProfileConnectionsView
    ) {
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: routeSection,
        timelineView: nextTimelineView,
        activeSettingsSection: nextSettingsSection as SettingsSection,
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

    if (nextTimelineView === 'bookmarks') {
      normalizedSelectedThread = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (requestedContext) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
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
      if (requestedThreadId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
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
    } else if (nextTimelineView !== 'bookmarks' && requestedContext === 'thread') {
      normalizedSelectedDirectMessagePeerPubkey = null;
      const threadReadyForNestedAuthor =
        requestedThreadId !== null &&
        requestedThreadId.length > 0 &&
        requestedThreadId === selectedThread &&
        thread.length > 0;

      if (!requestedThreadId) {
        shouldNormalize = true;
        normalizedSelectedThread = null;
        normalizedSelectedAuthorPubkey = null;
        if (selectedThread || selectedAuthorPubkey) {
          setSelectedThread(null);
          setThread([]);
          setReplyTarget(null);
          setRepostTarget(null);
          setSelectedAuthorPubkey(null);
          setSelectedAuthor(null);
          setAuthorError(null);
        }
      } else if (requestedThreadId !== selectedThread || thread.length === 0) {
        normalizedSelectedThread = requestedThreadId;
        void openThread(requestedThreadId, {
          historyMode: 'replace',
          normalizeOnEmpty: true,
          topic: nextTopic,
        });
      } else {
        normalizedSelectedThread = requestedThreadId;
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
    } else if (nextTimelineView !== 'bookmarks' && requestedContext === 'author') {
      normalizedSelectedThread = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (requestedThreadId) {
        shouldNormalize = true;
      }
      if (selectedThread) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
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
    } else if (nextTimelineView !== 'bookmarks' && requestedContext) {
      shouldNormalize = true;
      normalizedSelectedThread = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (selectedThread || selectedAuthorPubkey) {
        setSelectedThread(null);
        setThread([]);
        setReplyTarget(null);
        setRepostTarget(null);
        setSelectedAuthorPubkey(null);
        setSelectedAuthor(null);
        setAuthorError(null);
      }
      if (directMessagePaneOpen || selectedDirectMessagePeerPubkey) {
        setDirectMessagePaneOpen(false);
        setSelectedDirectMessagePeerPubkey(null);
        setDirectMessageError(null);
      }
    } else {
      if (requestedThreadId || requestedAuthorPubkey || requestedPeerPubkey) {
        shouldNormalize = true;
      }
      normalizedSelectedThread = null;
      normalizedSelectedAuthorPubkey = null;
      normalizedSelectedDirectMessagePeerPubkey = null;
      if (
        selectedThread ||
        selectedAuthorPubkey ||
        directMessagePaneOpen ||
        selectedDirectMessagePeerPubkey
      ) {
        setSelectedThread(null);
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
      );
    }

    if (shouldNormalize) {
      scheduleAnimationFrame(() => {
        syncRoute('replace', {
          activeTopic: nextTopic,
          composeTarget: privateComposeTarget(nextSelectedChannelId),
          primarySection: routeSection,
          profileConnectionsView: nextProfileConnectionsView,
          profileMode: nextProfileMode,
          selectedAuthorPubkey: normalizedSelectedAuthorPubkey,
          selectedDirectMessagePeerPubkey: normalizedSelectedDirectMessagePeerPubkey,
          selectedThread: normalizedSelectedThread,
          settingsOpen: nextSettingsOpen,
          settingsSection: nextSettingsSection as SettingsSection,
          timelineScope: privateTimelineScope(nextSelectedChannelId),
          timelineView: nextTimelineView,
        });
      });
    }
  }, [
    activeTopic,
    directMessagePaneOpen,
    joinedChannelsByTopic,
    loadTopics,
    location.pathname,
    location.search,
    navigate,
    openAuthorDetail,
    openDirectMessagePane,
    openThread,
    pendingRouteUrlRef,
    routeSection,
    scheduleAnimationFrame,
    selectedAuthor,
    selectedAuthorPubkey,
    selectedChannelIdByTopic,
    selectedDirectMessagePeerPubkey,
    selectedThread,
    setActiveTopic,
    setAuthorError,
    setComposeChannelByTopic,
    setDirectMessageError,
    setDirectMessagePaneOpen,
    setReplyTarget,
    setRepostTarget,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedChannelIdByTopic,
    setSelectedDirectMessagePeerPubkey,
    setSelectedThread,
    setShellChromeState,
    setThread,
    setTimelineScopeByTopic,
    shellChromeState.activePrimarySection,
    shellChromeState.activeSettingsSection,
    shellChromeState.profileConnectionsView,
    shellChromeState.profileMode,
    shellChromeState.settingsOpen,
    shellChromeState.timelineView,
    syncRoute,
    thread.length,
    trackedTopics,
  ]);

  return {
    routeSection,
    syncRoute,
    setNavOpen,
    setSettingsOpen,
    setPrimarySectionRef,
    focusPrimarySection,
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
