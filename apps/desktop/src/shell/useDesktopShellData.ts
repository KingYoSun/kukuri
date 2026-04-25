import {
  startTransition,
  useCallback,
  useRef,
  type MutableRefObject,
} from 'react';

import type {
  DesktopApi,
  DirectMessageMessageView,
  GameRoomView,
  JoinedPrivateChannelView,
  NotificationView,
  PostView,
} from '@/lib/api';

import { useDesktopShellDataEffects } from '@/shell/data/useDesktopShellDataEffects';
import { useDraftMediaHelpers } from '@/shell/data/useDraftMediaHelpers';
import { useQueuedLoadTopics } from '@/shell/data/useQueuedLoadTopics';
import {
  hasLoadedOlderAuthoritativePosts,
  mergeRefreshedVisiblePosts,
  mergeUniquePosts,
  postIdentityKey,
  uniquePostsByIdentity,
} from '@/shell/data/timelineMerge';
import { usePreviewableMediaAttachments } from '@/shell/data/usePreviewableMediaAttachments';
import {
  activeTimelineStorageKey,
  PUBLIC_TIMELINE_SCOPE,
  timelineScopeStorageKey,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  useDesktopShellStoreApi,
} from '@/shell/store';
import { THREAD_TIMELINE_LIMIT, VISIBLE_TIMELINE_LIMIT } from '@/shell/pagination';
import {
  authorViewFromDirectMessageConversation,
  communityNodesToDraftNodes,
  mergeCommunityNodeStatuses,
  mergeKnownAuthors,
  messageFromError,
  privateTimelineScope,
  profileInputFromProfile,
  seedPeersToEditorValue,
} from '@/shell/selectors';

type UseDesktopShellDataArgs = {
  api: DesktopApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
  loadTopicsRequestRef: MutableRefObject<number>;
  remoteObjectUrlRef: MutableRefObject<Map<string, string>>;
  draftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  directMessageDraftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  mediaFetchAttemptRef: MutableRefObject<Map<string, number>>;
  draftSequenceRef: MutableRefObject<number>;
};

const EMPTY_POSTS: PostView[] = [];
const EMPTY_GAME_ROOMS: GameRoomView[] = [];
const EMPTY_JOINED_CHANNELS: JoinedPrivateChannelView[] = [];
const EMPTY_DIRECT_MESSAGE_TIMELINE: DirectMessageMessageView[] = [];

export function useDesktopShellData({
  api,
  translate,
  loadTopicsRequestRef,
  remoteObjectUrlRef,
  draftPreviewUrlRef,
  directMessageDraftPreviewUrlRef,
  mediaFetchAttemptRef,
  draftSequenceRef,
}: UseDesktopShellDataArgs) {
  const storeApi = useDesktopShellStoreApi();
  const state = useDesktopShellStore();
  const {
    trackedTopics,
    activeTopic,
    selectedThread,
    gameRoomsByTopic,
    joinedChannelsByTopic,
    selectedChannelIdByTopic,
    mediaObjectUrls,
    localProfile,
    knownAuthorsByPubkey,
    profileTimeline,
    selectedAuthorTimeline,
    thread,
    ownedReactionAssets,
    bookmarkedReactionAssets,
    recentReactions,
    notifications,
    shellChromeState,
  } = state;
  const selectedDirectMessageTimeline =
    state.directMessageTimelineByPeer[state.selectedDirectMessagePeerPubkey ?? ''] ??
    EMPTY_DIRECT_MESSAGE_TIMELINE;
  const activeTimelineKey = activeTimelineStorageKey(state, activeTopic);
  const activePublicTimeline = state.publicTimelinesByTopic[activeTopic] ?? EMPTY_POSTS;
  const activeTimeline = state.timelinesByKey[activeTimelineKey] ?? EMPTY_POSTS;
  const activeGameRooms = gameRoomsByTopic[activeTopic] ?? EMPTY_GAME_ROOMS;
  const activeJoinedChannels = joinedChannelsByTopic[activeTopic] ?? EMPTY_JOINED_CHANNELS;
  const selectedPrivateChannelId = selectedChannelIdByTopic[activeTopic] ?? null;
  const selectedAuthorPubkey = state.selectedAuthorPubkey;
  const visibleRefreshInFlightRef = useRef(false);

  const setTimelinesByKey = useDesktopShellFieldSetter('timelinesByKey');
  const setTimelineNextCursorByKey = useDesktopShellFieldSetter('timelineNextCursorByKey');
  const setTimelineLoadingMoreByKey = useDesktopShellFieldSetter('timelineLoadingMoreByKey');
  const setPendingTimelineSnapshotsByKey = useDesktopShellFieldSetter(
    'pendingTimelineSnapshotsByKey'
  );
  const setPendingTimelineCountsByKey = useDesktopShellFieldSetter('pendingTimelineCountsByKey');
  const setPendingTimelineNextCursorByKey = useDesktopShellFieldSetter(
    'pendingTimelineNextCursorByKey'
  );
  const setPublicTimelinesByTopic = useDesktopShellFieldSetter('publicTimelinesByTopic');
  const setPublicTimelineNextCursorByTopic = useDesktopShellFieldSetter(
    'publicTimelineNextCursorByTopic'
  );
  const setLiveSessionsByTopic = useDesktopShellFieldSetter('liveSessionsByTopic');
  const setGameRoomsByTopic = useDesktopShellFieldSetter('gameRoomsByTopic');
  const setJoinedChannelsByTopic = useDesktopShellFieldSetter('joinedChannelsByTopic');
  const setChannelPanelStateByTopic = useDesktopShellFieldSetter('channelPanelStateByTopic');
  const setSelectedChannelIdByTopic = useDesktopShellFieldSetter('selectedChannelIdByTopic');
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setThread = useDesktopShellFieldSetter('thread');
  const setThreadNextCursorById = useDesktopShellFieldSetter('threadNextCursorById');
  const setThreadLoadingMoreById = useDesktopShellFieldSetter('threadLoadingMoreById');
  const setLocalPeerTicket = useDesktopShellFieldSetter('localPeerTicket');
  const setDiscoveryConfig = useDesktopShellFieldSetter('discoveryConfig');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setDiscoveryError = useDesktopShellFieldSetter('discoveryError');
  const setCommunityNodeConfig = useDesktopShellFieldSetter('communityNodeConfig');
  const setCommunityNodeStatuses = useDesktopShellFieldSetter('communityNodeStatuses');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setCommunityNodeError = useDesktopShellFieldSetter('communityNodeError');
  const setMediaObjectUrls = useDesktopShellFieldSetter('mediaObjectUrls');
  const setSyncStatus = useDesktopShellFieldSetter('syncStatus');
  const setLocalProfile = useDesktopShellFieldSetter('localProfile');
  const setProfileTimeline = useDesktopShellFieldSetter('profileTimeline');
  const setProfileTimelineNextCursor = useDesktopShellFieldSetter('profileTimelineNextCursor');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const setSocialConnections = useDesktopShellFieldSetter('socialConnections');
  const setSocialConnectionsPanelState = useDesktopShellFieldSetter('socialConnectionsPanelState');
  const setOwnedReactionAssets = useDesktopShellFieldSetter('ownedReactionAssets');
  const setBookmarkedReactionAssets = useDesktopShellFieldSetter('bookmarkedReactionAssets');
  const setBookmarkedPosts = useDesktopShellFieldSetter('bookmarkedPosts');
  const setRecentReactions = useDesktopShellFieldSetter('recentReactions');
  const setProfileDraft = useDesktopShellFieldSetter('profileDraft');
  const setProfileError = useDesktopShellFieldSetter('profileError');
  const setProfilePanelState = useDesktopShellFieldSetter('profilePanelState');
  const setSelectedAuthor = useDesktopShellFieldSetter('selectedAuthor');
  const setSelectedAuthorTimeline = useDesktopShellFieldSetter('selectedAuthorTimeline');
  const setSelectedAuthorTimelineNextCursor = useDesktopShellFieldSetter(
    'selectedAuthorTimelineNextCursor'
  );
  const setAuthorError = useDesktopShellFieldSetter('authorError');
  const setNotifications = useDesktopShellFieldSetter('notifications');
  const setNotificationStatus = useDesktopShellFieldSetter('notificationStatus');
  const setNotificationPanelState = useDesktopShellFieldSetter('notificationPanelState');
  const setNotificationAutoReadError = useDesktopShellFieldSetter('notificationAutoReadError');
  const setDirectMessages = useDesktopShellFieldSetter('directMessages');
  const setDirectMessageTimelineByPeer = useDesktopShellFieldSetter('directMessageTimelineByPeer');
  const setDirectMessageTimelineNextCursorByPeer = useDesktopShellFieldSetter(
    'directMessageTimelineNextCursorByPeer'
  );
  const setDirectMessageStatusByPeer = useDesktopShellFieldSetter('directMessageStatusByPeer');
  const setDirectMessageError = useDesktopShellFieldSetter('directMessageError');
  const setLivePanelStateByTopic = useDesktopShellFieldSetter('livePanelStateByTopic');
  const setGamePanelStateByTopic = useDesktopShellFieldSetter('gamePanelStateByTopic');
  const setGameDrafts = useDesktopShellFieldSetter('gameDrafts');
  const setReactionPanelState = useDesktopShellFieldSetter('reactionPanelState');
  const setError = useDesktopShellFieldSetter('error');

  const previewableMediaAttachments = usePreviewableMediaAttachments({
    activeTimeline,
    activePublicTimeline,
    profileTimeline,
    selectedAuthorTimeline,
    thread,
    selectedDirectMessageTimeline,
    ownedReactionAssets,
    bookmarkedReactionAssets,
    recentReactions,
    localProfile,
    knownAuthorsByPubkey,
    notifications,
  });

  const clearPendingTimeline = useCallback(
    (key: string) => {
      setPendingTimelineSnapshotsByKey((current) => {
        if (!current[key]) {
          return current;
        }
        const next = { ...current };
        delete next[key];
        return next;
      });
      setPendingTimelineCountsByKey((current) => {
        if (!current[key]) {
          return current;
        }
        const next = { ...current };
        delete next[key];
        return next;
      });
      setPendingTimelineNextCursorByKey((current) => {
        if (!(key in current)) {
          return current;
        }
        const next = { ...current };
        delete next[key];
        return next;
      });
    },
    [
      setPendingTimelineCountsByKey,
      setPendingTimelineNextCursorByKey,
      setPendingTimelineSnapshotsByKey,
    ]
  );

  const applyPendingTimeline = useCallback(
    (
      topic: string,
      scope = storeApi.getState().timelineScopeByTopic[topic] ??
        privateTimelineScope(storeApi.getState().selectedChannelIdByTopic[topic] ?? null)
    ) => {
      const key = timelineScopeStorageKey(topic, scope);
      const currentState = storeApi.getState();
      const pendingItems = currentState.pendingTimelineSnapshotsByKey[key];
      if (!pendingItems || pendingItems.length === 0) {
        return false;
      }
      const currentTimelinePosts = currentState.timelinesByKey[key] ?? EMPTY_POSTS;
      const preserveOlderPages = hasLoadedOlderAuthoritativePosts(currentTimelinePosts, pendingItems);
      startTransition(() => {
        setTimelinesByKey((current) => ({
          ...current,
          [key]: mergeRefreshedVisiblePosts(
            current[key] ?? EMPTY_POSTS,
            pendingItems,
            preserveOlderPages
          ),
        }));
        setTimelineNextCursorByKey((current) => ({
          ...current,
          [key]: currentState.pendingTimelineNextCursorByKey[key] ?? null,
        }));
      });
      clearPendingTimeline(key);
      return true;
    },
    [
      clearPendingTimeline,
      setTimelineNextCursorByKey,
      setTimelinesByKey,
      storeApi,
    ]
  );

  const refreshVisibleShellData = useCallback(
    async (
      topic: string,
      currentThread: string | null,
      mode: 'apply' | 'buffer' = 'buffer'
    ) => {
      const requestId = loadTopicsRequestRef.current + 1;
      loadTopicsRequestRef.current = requestId;
      const requestState = storeApi.getState();
      const selectedChannelId = requestState.selectedChannelIdByTopic[topic] ?? null;
      const timelineScope = privateTimelineScope(selectedChannelId);
      const timelineKey = timelineScopeStorageKey(topic, timelineScope);

      const [
        timelineResult,
        publicTimelineResult,
        joinedChannelsResult,
        threadViewResult,
        statusResult,
        communityNodeStatusesResult,
      ] = await Promise.allSettled([
          api.listTimeline(topic, null, VISIBLE_TIMELINE_LIMIT, timelineScope),
          api.listTimeline(topic, null, VISIBLE_TIMELINE_LIMIT, PUBLIC_TIMELINE_SCOPE),
          api.listJoinedPrivateChannels(topic),
          currentThread
            ? api.listThread(topic, currentThread, null, THREAD_TIMELINE_LIMIT)
            : Promise.resolve(null),
          api.getSyncStatus(),
          api.getCommunityNodeStatuses(),
        ]);

      if (requestId !== loadTopicsRequestRef.current) {
        return;
      }

      const firstCoreFailure = [
        timelineResult,
        publicTimelineResult,
        joinedChannelsResult,
        threadViewResult,
        statusResult,
      ].find((result) => result.status === 'rejected');

      startTransition(() => {
        const currentState = storeApi.getState();

        if (timelineResult.status === 'fulfilled') {
          const timeline = timelineResult.value;
          const normalizedTimelineItems = uniquePostsByIdentity(timeline.items);
          const baselinePosts = currentState.timelinesByKey[timelineKey] ?? EMPTY_POSTS;
          const preserveTimelinePages =
            mode === 'buffer' &&
            hasLoadedOlderAuthoritativePosts(baselinePosts, normalizedTimelineItems);
          const resolvedTimelineCursor = preserveTimelinePages
            ? (currentState.timelineNextCursorByKey[timelineKey] ?? null)
            : (timeline.next_cursor ?? null);
          const visiblePostIds = new Set(baselinePosts.map((post) => postIdentityKey(post)));
          const authoritativeIds = new Set(
            baselinePosts
              .filter((post) => !post.local_state)
              .map((post) => postIdentityKey(post))
          );
          const hasAuthoritativeBaseline = authoritativeIds.size > 0;
          const pendingTimelineItems = normalizedTimelineItems.filter(
            (post) => !visiblePostIds.has(postIdentityKey(post))
          );
          const pendingCount = pendingTimelineItems.length;
          const shouldBuffer = mode === 'buffer' && hasAuthoritativeBaseline && pendingCount > 0;

          if (shouldBuffer) {
            setPendingTimelineSnapshotsByKey((current) => ({
              ...current,
              [timelineKey]: normalizedTimelineItems,
            }));
            setPendingTimelineCountsByKey((current) => ({
              ...current,
              [timelineKey]: pendingCount,
            }));
            setPendingTimelineNextCursorByKey((current) => ({
              ...current,
              [timelineKey]: resolvedTimelineCursor,
            }));
          } else {
            setTimelinesByKey((current) => ({
              ...current,
              [timelineKey]: mergeRefreshedVisiblePosts(
                current[timelineKey] ?? EMPTY_POSTS,
                normalizedTimelineItems,
                preserveTimelinePages
              ),
            }));
            setTimelineNextCursorByKey((current) => ({
              ...current,
              [timelineKey]: resolvedTimelineCursor,
            }));
            clearPendingTimeline(timelineKey);
          }
        }

        if (publicTimelineResult.status === 'fulfilled') {
          const publicTimeline = publicTimelineResult.value;
          const baselinePublicTimeline = currentState.publicTimelinesByTopic[topic] ?? EMPTY_POSTS;
          const preservePublicTimelinePages =
            mode === 'buffer' &&
            hasLoadedOlderAuthoritativePosts(baselinePublicTimeline, publicTimeline.items);
          const resolvedPublicTimelineCursor = preservePublicTimelinePages
            ? (currentState.publicTimelineNextCursorByTopic[topic] ?? null)
            : (publicTimeline.next_cursor ?? null);
          setPublicTimelinesByTopic((current) => ({
            ...current,
            [topic]: mergeRefreshedVisiblePosts(
              current[topic] ?? EMPTY_POSTS,
              publicTimeline.items,
              preservePublicTimelinePages
            ),
          }));
          setPublicTimelineNextCursorByTopic((current) => ({
            ...current,
            [topic]: resolvedPublicTimelineCursor,
          }));
        }

        if (joinedChannelsResult.status === 'fulfilled') {
          setJoinedChannelsByTopic((current) => ({
            ...current,
            [topic]: joinedChannelsResult.value,
          }));
          setChannelPanelStateByTopic((current) => ({
            ...current,
            [topic]: {
              status: 'ready',
              error: null,
            },
          }));
        } else {
          setChannelPanelStateByTopic((current) => ({
            ...current,
            [topic]: {
              status: 'error',
              error: messageFromError(
                joinedChannelsResult.reason,
                translate('common:errors.failedToLoadPrivateChannels')
              ),
            },
          }));
        }

        if (currentThread) {
          if (threadViewResult.status === 'fulfilled') {
            const threadView = threadViewResult.value;
            const incomingThreadItems = threadView?.items ?? [];
            const currentThreadPosts = currentState.thread;
            const preserveThreadPages =
              mode === 'buffer' &&
              hasLoadedOlderAuthoritativePosts(currentThreadPosts, incomingThreadItems);
            const resolvedThreadCursor = preserveThreadPages
              ? (currentState.threadNextCursorById[currentThread] ?? null)
              : (threadView?.next_cursor ?? null);
            setThread((current) =>
              mergeRefreshedVisiblePosts(current, incomingThreadItems, preserveThreadPages)
            );
            setThreadNextCursorById((current) => ({
              ...current,
              [currentThread]: resolvedThreadCursor,
            }));
          }
        } else {
          setThread([]);
        }

        if (statusResult.status === 'fulfilled') {
          setSyncStatus(statusResult.value);
        }

        if (communityNodeStatusesResult.status === 'fulfilled') {
          setCommunityNodeStatuses((current) =>
            mergeCommunityNodeStatuses(current, communityNodeStatusesResult.value)
          );
        }

        setError(
          firstCoreFailure && firstCoreFailure.status === 'rejected'
            ? messageFromError(firstCoreFailure.reason, translate('common:errors.failedToLoadTopic'))
            : null
        );
      });
    },
    [
      api,
      clearPendingTimeline,
      loadTopicsRequestRef,
      setCommunityNodeStatuses,
      setError,
      setChannelPanelStateByTopic,
      setJoinedChannelsByTopic,
      setPendingTimelineCountsByKey,
      setPendingTimelineNextCursorByKey,
      setPendingTimelineSnapshotsByKey,
      setPublicTimelineNextCursorByTopic,
      setPublicTimelinesByTopic,
      setSyncStatus,
      setThread,
      setThreadNextCursorById,
      setTimelineNextCursorByKey,
      setTimelinesByKey,
      storeApi,
      translate,
    ]
  );

  const loadMoreTimeline = useCallback(
    async (topic: string) => {
      const currentState = storeApi.getState();
      const timelineKey = activeTimelineStorageKey(currentState, topic);
      const cursor = currentState.timelineNextCursorByKey[timelineKey] ?? null;
      if (!cursor || currentState.timelineLoadingMoreByKey[timelineKey]) {
        return;
      }
      const selectedChannelId = currentState.selectedChannelIdByTopic[topic] ?? null;
      setTimelineLoadingMoreByKey((current) => ({
        ...current,
        [timelineKey]: true,
      }));
      try {
        const timeline = await api.listTimeline(
          topic,
          cursor,
          VISIBLE_TIMELINE_LIMIT,
          privateTimelineScope(selectedChannelId)
        );
        startTransition(() => {
          setTimelinesByKey((current) => ({
            ...current,
            [timelineKey]: mergeUniquePosts(current[timelineKey] ?? EMPTY_POSTS, timeline.items),
          }));
          setTimelineNextCursorByKey((current) => ({
            ...current,
            [timelineKey]: timeline.next_cursor ?? null,
          }));
        });
      } finally {
        setTimelineLoadingMoreByKey((current) => ({
          ...current,
          [timelineKey]: false,
        }));
      }
    },
    [
      api,
      setTimelineLoadingMoreByKey,
      setTimelineNextCursorByKey,
      setTimelinesByKey,
      storeApi,
    ]
  );

  const loadMoreThread = useCallback(
    async (topic: string, threadId: string) => {
      const currentState = storeApi.getState();
      const cursor = currentState.threadNextCursorById[threadId] ?? null;
      if (!cursor || currentState.threadLoadingMoreById[threadId]) {
        return;
      }
      setThreadLoadingMoreById((current) => ({
        ...current,
        [threadId]: true,
      }));
      try {
        const threadView = await api.listThread(topic, threadId, cursor, THREAD_TIMELINE_LIMIT);
        startTransition(() => {
          setThread((current) => mergeUniquePosts(current, threadView.items));
          setThreadNextCursorById((current) => ({
            ...current,
            [threadId]: threadView.next_cursor ?? null,
          }));
        });
      } finally {
        setThreadLoadingMoreById((current) => ({
          ...current,
          [threadId]: false,
        }));
      }
    },
    [
      api,
      setThread,
      setThreadLoadingMoreById,
      setThreadNextCursorById,
      storeApi,
    ]
  );

  const loadReactionCatalogData = useCallback(async () => {
    try {
      const [ownedAssets, bookmarkedAssets, recent] = await Promise.all([
        api.listMyCustomReactionAssets(),
        api.listBookmarkedCustomReactions(),
        api.listRecentReactions(8),
      ]);
      startTransition(() => {
        setOwnedReactionAssets(ownedAssets);
        setBookmarkedReactionAssets(bookmarkedAssets);
        setRecentReactions(recent);
        setReactionPanelState({ status: 'ready', error: null });
      });
    } catch (error) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(error, translate('common:errors.failedToLoadSettings')),
      });
    }
  }, [
    api,
    setBookmarkedReactionAssets,
    setOwnedReactionAssets,
    setReactionPanelState,
    setRecentReactions,
    translate,
  ]);

  const runLoadTopics = useCallback(
    async (_currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      await refreshVisibleShellData(currentActiveTopic, currentThread, 'apply');

      const currentState = storeApi.getState();
      const selectedChannelId = currentState.selectedChannelIdByTopic[currentActiveTopic] ?? null;
      const selectedAuthorPubkey = currentState.selectedAuthorPubkey;
      const {
        activePrimarySection,
        activeSettingsSection,
        settingsOpen,
        timelineView,
      } = currentState.shellChromeState;

      const tasks: Promise<void>[] = [];

      if (activePrimarySection === 'live') {
        tasks.push(
          api
            .listLiveSessions(currentActiveTopic, privateTimelineScope(selectedChannelId))
            .then((sessions) => {
              startTransition(() => {
                setLiveSessionsByTopic((current) => ({
                  ...current,
                  [currentActiveTopic]: sessions,
                }));
                setLivePanelStateByTopic((current) => ({
                  ...current,
                  [currentActiveTopic]: { status: 'ready', error: null },
                }));
              });
            })
            .catch((error) => {
              setLivePanelStateByTopic((current) => ({
                ...current,
                [currentActiveTopic]: {
                  status: 'error',
                  error: messageFromError(
                    error,
                    translate('common:errors.failedToLoadLiveSessions')
                  ),
                },
              }));
            })
        );
      }

      if (activePrimarySection === 'game') {
        tasks.push(
          api
            .listGameRooms(currentActiveTopic, privateTimelineScope(selectedChannelId))
            .then((rooms) => {
              startTransition(() => {
                setGameRoomsByTopic((current) => ({
                  ...current,
                  [currentActiveTopic]: rooms,
                }));
                setGamePanelStateByTopic((current) => ({
                  ...current,
                  [currentActiveTopic]: { status: 'ready', error: null },
                }));
              });
            })
            .catch((error) => {
              setGamePanelStateByTopic((current) => ({
                ...current,
                [currentActiveTopic]: {
                  status: 'error',
                  error: messageFromError(error, translate('common:errors.failedToLoadGameRooms')),
                },
              }));
            })
        );
      }

      if (activePrimarySection === 'profile') {
        tasks.push(
          Promise.all([
            api.getMyProfile(),
            api.listSocialConnections('following'),
            api.listSocialConnections('followed'),
            api.listSocialConnections('muted'),
          ])
            .then(async ([profile, following, followed, muted]) => {
              const timeline = await api.listProfileTimeline(profile.pubkey, null, VISIBLE_TIMELINE_LIMIT);
              startTransition(() => {
                setLocalProfile(profile);
                if (!storeApi.getState().profileDirty) {
                  setProfileDraft(profileInputFromProfile(profile));
                }
                setProfileTimeline(timeline.items);
                setProfileTimelineNextCursor(timeline.next_cursor ?? null);
                setProfileError(null);
                setProfilePanelState({ status: 'ready', error: null });
                setSocialConnections({ following, followed, muted });
                setKnownAuthorsByPubkey((current) =>
                  mergeKnownAuthors(current, [...following, ...followed, ...muted])
                );
                setSocialConnectionsPanelState({ status: 'ready', error: null });
              });
            })
            .catch((error) => {
              const message = messageFromError(error, translate('common:errors.failedToLoadProfile'));
              setProfileError(message);
              setProfilePanelState({ status: 'error', error: message });
            })
        );
      }

      if (selectedAuthorPubkey) {
        tasks.push(
          Promise.all([
            api.getAuthorSocialView(selectedAuthorPubkey),
            api.listProfileTimeline(selectedAuthorPubkey, null, VISIBLE_TIMELINE_LIMIT),
          ])
            .then(([author, timeline]) => {
              startTransition(() => {
                setSelectedAuthor(author);
                setSelectedAuthorTimeline(timeline.items);
                setSelectedAuthorTimelineNextCursor(timeline.next_cursor ?? null);
                setAuthorError(null);
                if (author) {
                  setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [author]));
                }
              });
            })
            .catch((error) => {
              setAuthorError(messageFromError(error, translate('common:errors.failedToLoadAuthor')));
            })
        );
      }

      if (activePrimarySection === 'messages' || currentState.directMessagePaneOpen) {
        tasks.push(
          api
            .listDirectMessages()
            .then(async (directMessages) => {
              startTransition(() => {
                setDirectMessages(directMessages);
                setKnownAuthorsByPubkey((current) =>
                  mergeKnownAuthors(
                    current,
                    directMessages.map(authorViewFromDirectMessageConversation)
                  )
                );
              });
              const selectedPeerPubkey = storeApi.getState().selectedDirectMessagePeerPubkey;
              if (!selectedPeerPubkey) {
                setDirectMessageError(null);
                return;
              }
              const [timelineResult, statusResult] = await Promise.allSettled([
                api.listDirectMessageMessages(selectedPeerPubkey, null, VISIBLE_TIMELINE_LIMIT),
                api.getDirectMessageStatus(selectedPeerPubkey),
              ]);
              startTransition(() => {
                if (timelineResult.status === 'fulfilled') {
                  setDirectMessageTimelineByPeer((current) => ({
                    ...current,
                    [selectedPeerPubkey]: timelineResult.value.items,
                  }));
                  setDirectMessageTimelineNextCursorByPeer((current) => ({
                    ...current,
                    [selectedPeerPubkey]: timelineResult.value.next_cursor ?? null,
                  }));
                }
                if (statusResult.status === 'fulfilled') {
                  setDirectMessageStatusByPeer((current) => ({
                    ...current,
                    [selectedPeerPubkey]: statusResult.value,
                  }));
                }
                setDirectMessageError(
                  timelineResult.status === 'fulfilled' && statusResult.status === 'fulfilled'
                    ? null
                    : messageFromError(
                        timelineResult.status === 'rejected'
                          ? timelineResult.reason
                          : statusResult.status === 'rejected'
                            ? statusResult.reason
                            : null,
                        'failed to load direct messages'
                      )
                );
              });
            })
            .catch((error) => {
              setDirectMessageError(messageFromError(error, 'failed to load direct messages'));
            })
        );
      }

      if (activePrimarySection === 'notifications') {
        tasks.push(
          Promise.all([api.getNotificationStatus(), api.listNotifications()])
            .then(async ([status, notificationItems]) => {
              let nextNotifications: NotificationView[] = notificationItems;
              let nextStatus = status;
              if (notificationItems.some((notification) => !notification.read_at)) {
                try {
                  nextStatus = await api.markAllNotificationsRead();
                  const readAt = Date.now();
                  nextNotifications = notificationItems.map((notification) =>
                    notification.read_at ? notification : { ...notification, read_at: readAt }
                  );
                  setNotificationAutoReadError(null);
                } catch (notificationReadError) {
                  setNotificationAutoReadError(
                    messageFromError(
                      notificationReadError,
                      translate('shell:notifications.errors.failedAutoRead')
                    )
                  );
                }
              }
              startTransition(() => {
                setNotificationStatus(nextStatus);
                setNotifications(nextNotifications);
                setNotificationPanelState({ status: 'ready', error: null });
              });
            })
            .catch((error) => {
              setNotificationPanelState({
                status: 'error',
                error: messageFromError(error, translate('shell:notifications.errors.failedToLoad')),
              });
            })
        );
      }

      if (activePrimarySection === 'timeline' && timelineView === 'bookmarks') {
        tasks.push(
          api
            .listBookmarkedPosts()
            .then((bookmarks) => {
              setBookmarkedPosts(bookmarks);
            })
            .catch(() => undefined)
        );
      }

      if (settingsOpen) {
        if (activeSettingsSection === 'connectivity') {
          tasks.push(
            api
              .getLocalPeerTicket()
              .then((ticket) => {
                setLocalPeerTicket(ticket);
              })
              .catch(() => undefined)
          );
        }

        if (activeSettingsSection === 'discovery') {
          tasks.push(
            api
              .getDiscoveryConfig()
              .then((config) => {
                setDiscoveryConfig(config);
                if (!storeApi.getState().discoveryEditorDirty) {
                  setDiscoverySeedInput(seedPeersToEditorValue(config));
                }
                setDiscoveryError(null);
              })
              .catch((error) => {
                setDiscoveryError(
                  messageFromError(error, translate('common:errors.failedToLoadSettings'))
                );
              })
          );
        }

        if (activeSettingsSection === 'community-node') {
          tasks.push(
            Promise.all([api.getCommunityNodeConfig(), api.getCommunityNodeStatuses()])
              .then(([config, statuses]) => {
                startTransition(() => {
                  setCommunityNodeConfig(config);
                  if (!storeApi.getState().communityNodeEditorDirty) {
                    setCommunityNodeInput(communityNodesToDraftNodes(config));
                  }
                  setCommunityNodeStatuses((current) =>
                    mergeCommunityNodeStatuses(current, statuses)
                  );
                  setCommunityNodeError(null);
                });
              })
              .catch((error) => {
                setCommunityNodeError(
                  messageFromError(error, translate('common:errors.failedToLoadSettings'))
                );
              })
          );
        }

        if (activeSettingsSection === 'reactions') {
          tasks.push(
            Promise.all([api.listBookmarkedPosts(), loadReactionCatalogData()])
              .then(([bookmarkedPosts]) => {
                startTransition(() => {
                  setBookmarkedPosts(bookmarkedPosts);
                });
              })
              .catch((error) => {
                setReactionPanelState({
                  status: 'error',
                  error: messageFromError(error, translate('common:errors.failedToLoadSettings')),
                });
              })
          );
        }
      }

      await Promise.allSettled(tasks);
    },
    [
      api,
      setAuthorError,
      setBookmarkedPosts,
      setCommunityNodeConfig,
      setCommunityNodeError,
      setCommunityNodeInput,
      setCommunityNodeStatuses,
      setDirectMessages,
      setDirectMessageError,
      setDirectMessageTimelineNextCursorByPeer,
      setDirectMessageTimelineByPeer,
      setDirectMessageStatusByPeer,
      setDiscoveryConfig,
      setDiscoveryError,
      setDiscoverySeedInput,
      setGamePanelStateByTopic,
      setGameRoomsByTopic,
      setKnownAuthorsByPubkey,
      setLivePanelStateByTopic,
      setLiveSessionsByTopic,
      setLocalPeerTicket,
      setLocalProfile,
      setNotifications,
      setNotificationAutoReadError,
      setNotificationPanelState,
      setNotificationStatus,
      setProfileDraft,
      setProfileError,
      setProfilePanelState,
      setProfileTimelineNextCursor,
      setProfileTimeline,
      setReactionPanelState,
      setSelectedAuthor,
      setSelectedAuthorTimelineNextCursor,
      setSelectedAuthorTimeline,
      setSocialConnections,
      setSocialConnectionsPanelState,
      loadReactionCatalogData,
      refreshVisibleShellData,
      storeApi,
      translate,
    ]
  );

  const loadTopics = useQueuedLoadTopics(runLoadTopics);

  const refreshVisibleTimelineAfterPublish = useCallback(
    async (topic: string, currentThread: string | null) => {
      await refreshVisibleShellData(topic, currentThread, 'apply');
    },
    [refreshVisibleShellData]
  );

  const refreshTimelineFeed = useCallback(
    async (topic: string, currentThread: string | null) => {
      if (applyPendingTimeline(topic)) {
        return;
      }
      await refreshVisibleShellData(topic, currentThread, 'apply');
    },
    [applyPendingTimeline, refreshVisibleShellData]
  );

  useDesktopShellDataEffects({
    api,
    translate,
    storeApi,
    trackedTopics,
    activeTopic,
    selectedThread,
    activeGameRooms,
    activeJoinedChannels,
    selectedPrivateChannelId,
    mediaObjectUrls,
    shellChromeState,
    selectedAuthorPubkey,
    previewableMediaAttachments,
    remoteObjectUrlRef,
    draftPreviewUrlRef,
    directMessageDraftPreviewUrlRef,
    mediaFetchAttemptRef,
    visibleRefreshInFlightRef,
    loadTopics,
    refreshVisibleShellData,
    setNotificationStatus,
    setLocalProfile,
    setProfileDraft,
    setKnownAuthorsByPubkey,
    setProfileTimeline,
    setProfileTimelineNextCursor,
    setProfileError,
    setProfilePanelState,
    setSocialConnections,
    setSocialConnectionsPanelState,
    setSelectedAuthor,
    setSelectedAuthorTimeline,
    setSelectedAuthorTimelineNextCursor,
    setAuthorError,
    setDirectMessages,
    setDirectMessageTimelineByPeer,
    setDirectMessageTimelineNextCursorByPeer,
    setDirectMessageStatusByPeer,
    setDirectMessageError,
    setNotifications,
    setNotificationPanelState,
    setNotificationAutoReadError,
    setGameDrafts,
    setSelectedChannelIdByTopic,
    setComposeChannelByTopic,
    setTimelineScopeByTopic,
    setMediaObjectUrls,
  });

  const {
    rememberDraftPreview,
    releaseDraftPreview,
    releaseAllDraftPreviews,
    rememberDirectMessageDraftPreview,
    releaseDirectMessageDraftPreview,
    releaseAllDirectMessageDraftPreviews,
    buildImageDraftItem,
    buildVideoDraftItem,
  } = useDraftMediaHelpers({
    draftPreviewUrlRef,
    directMessageDraftPreviewUrlRef,
    draftSequenceRef,
  });

  return {
    loadTopics,
    refreshVisibleShellData,
    refreshVisibleTimelineAfterPublish,
    refreshTimelineFeed,
    applyPendingTimeline,
    loadReactionCatalogData,
    loadMoreTimeline,
    loadMoreThread,
    rememberDraftPreview,
    releaseDraftPreview,
    releaseAllDraftPreviews,
    rememberDirectMessageDraftPreview,
    releaseDirectMessageDraftPreview,
    releaseAllDirectMessageDraftPreviews,
    buildImageDraftItem,
    buildVideoDraftItem,
  };
}
