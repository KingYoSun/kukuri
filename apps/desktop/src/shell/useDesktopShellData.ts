import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type MutableRefObject,
} from 'react';

import type {
  DesktopApi,
  DirectMessageMessageView,
  GameRoomView,
  JoinedPrivateChannelView,
  PostView,
} from '@/lib/api';

import { removeRecordEntry, setRecordEntry, updateRecordEntry } from '@/shell/stateUpdates';
import { useConnectivityStatusRefresh } from '@/shell/data/useConnectivityStatusRefresh';
import { useDesktopShellDataEffects } from '@/shell/data/useDesktopShellDataEffects';
import {
  activeWorkspaceScope,
  columnIdentityId,
  openTransientColumn,
} from '@/shell/slices/workspace';
import { useDraftMediaHelpers } from '@/shell/data/useDraftMediaHelpers';
import { useDesktopShellSectionLoaders } from '@/shell/data/loaders/useDesktopShellSectionLoaders';
import { useQueuedLoadTopics } from '@/shell/data/useQueuedLoadTopics';
import {
  hasLoadedOlderAuthoritativePosts,
  mergeRefreshedVisiblePosts,
  mergeUniquePosts,
  postIdentityKey,
  uniquePostsByIdentity,
} from '@/shell/data/timelineMerge';
import { usePreviewableMediaAttachments } from '@/shell/data/usePreviewableMediaAttachments';
import { isAdultLabeledPost } from '@/shell/media';
import {
  activeTimelineStorageKey,
  PUBLIC_TIMELINE_SCOPE,
  timelineScopeStorageKey,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  useDesktopShellStoreApi,
} from '@/shell/store';
import { THREAD_TIMELINE_LIMIT, VISIBLE_TIMELINE_LIMIT } from '@/shell/pagination';
import { useShallow } from 'zustand/react/shallow';
import {
  mergeKnownAuthors,
  messageFromError,
  privateTimelineScope,
} from '@/shell/presentation';
import { selectShellDataSlice } from '@/shell/storeSelectors';

type UseDesktopShellDataArgs = {
  api: DesktopApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
  loadTopicsRequestRef: MutableRefObject<Map<string, number>>;
  remoteObjectUrlRef: MutableRefObject<Map<string, string>>;
  draftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  directMessageDraftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  mediaFetchAttemptRef: MutableRefObject<Map<string, number>>;
  draftSequenceRef: MutableRefObject<number>;
  /// 表示中(viewport 内)の Column id 列。背景 Timeline Column の定期 refresh(Issue #765)に使う。
  visibleColumnIdsRef?: MutableRefObject<string[]>;
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
  visibleColumnIdsRef,
}: UseDesktopShellDataArgs) {
  const storeApi = useDesktopShellStoreApi();
  const state = useDesktopShellStore(useShallow(selectShellDataSlice));
  const {
    trackedTopics,
    activeTopic,
    adultContentEnabled,
    selectedThread,
    gameRoomsByScopeKey,
    joinedChannelsByTopic,
    selectedChannelIdByTopic,
    mediaObjectUrls,
    localProfile,
    knownAuthorsByPubkey,
    profileTimeline,
    selectedAuthorTimeline,
    threadsById,
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
  const activePublicTimeline =
    state.timelinesByKey[timelineScopeStorageKey(activeTopic, PUBLIC_TIMELINE_SCOPE)] ?? EMPTY_POSTS;
  const activeTimeline = state.timelinesByKey[activeTimelineKey] ?? EMPTY_POSTS;
  const activeScope = activeWorkspaceScope(state.workspaceState);
  const activeGameRooms =
    gameRoomsByScopeKey[timelineScopeStorageKey(activeScope.topicId, privateTimelineScope(activeScope.channelId))] ??
    EMPTY_GAME_ROOMS;
  const activeJoinedChannels = joinedChannelsByTopic[activeTopic] ?? EMPTY_JOINED_CHANNELS;
  const selectedPrivateChannelId = selectedChannelIdByTopic[activeTopic] ?? null;
  const selectedAuthorPubkey = state.selectedAuthorPubkey;
  const thread = selectedThread ? threadsById[selectedThread] ?? EMPTY_POSTS : EMPTY_POSTS;
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
  const setJoinedChannelsByTopic = useDesktopShellFieldSetter('joinedChannelsByTopic');
  const setChannelPanelStateByTopic = useDesktopShellFieldSetter('channelPanelStateByTopic');
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');
  const setSelectedChannelIdByTopic = (
    value:
      | Record<string, string | null>
      | ((current: Record<string, string | null>) => Record<string, string | null>)
  ) => {
    const currentScope = activeWorkspaceScope(storeApi.getState().workspaceState);
    const currentProjection = { [currentScope.topicId]: currentScope.channelId };
    const nextProjection = typeof value === 'function' ? value(currentProjection) : value;
    const topicId = Object.keys(nextProjection).find(
      (topic) => nextProjection[topic] !== currentProjection[topic]
    ) ?? currentScope.topicId;
    const scope = { topicId, channelId: nextProjection[topicId] ?? null };
    setWorkspaceState((current) =>
      openTransientColumn(current, {
        id: columnIdentityId('timeline', scope),
        kind: 'timeline',
        scope,
        pinned: false,
      })
    );
  };
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setThreadsById = useDesktopShellFieldSetter('threadsById');
  const setThreadNextCursorById = useDesktopShellFieldSetter('threadNextCursorById');
  const setThreadLoadingMoreById = useDesktopShellFieldSetter('threadLoadingMoreById');
  const setCommunityNodeStatuses = useDesktopShellFieldSetter('communityNodeStatuses');
  const setMediaObjectUrls = useDesktopShellFieldSetter('mediaObjectUrls');
  const setSyncStatus = useDesktopShellFieldSetter('syncStatus');
  const setLocalProfile = useDesktopShellFieldSetter('localProfile');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const setOwnedReactionAssets = useDesktopShellFieldSetter('ownedReactionAssets');
  const setBookmarkedReactionAssets = useDesktopShellFieldSetter('bookmarkedReactionAssets');
  const setRecentReactions = useDesktopShellFieldSetter('recentReactions');
  const setProfileDraft = useDesktopShellFieldSetter('profileDraft');
  const setNotifications = useDesktopShellFieldSetter('notifications');
  const setNotificationStatus = useDesktopShellFieldSetter('notificationStatus');
  const setGameDrafts = useDesktopShellFieldSetter('gameDrafts');
  const setReactionPanelState = useDesktopShellFieldSetter('reactionPanelState');
  const setError = useDesktopShellFieldSetter('error');

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'game') {
      return;
    }
    const missingHostPubkeys = Array.from(
      new Set(
        activeGameRooms
          .map((room) => room.host_pubkey)
          .filter(
            (pubkey) =>
              pubkey &&
              pubkey !== localProfile?.pubkey &&
              pubkey !== state.syncStatus.local_author_pubkey &&
              !knownAuthorsByPubkey[pubkey]
          )
      )
    );
    if (missingHostPubkeys.length === 0) {
      return;
    }
    let disposed = false;
    void Promise.all(
      missingHostPubkeys.map((pubkey) => api.getAuthorSocialView(pubkey).catch(() => null))
    ).then((authors) => {
      if (disposed) {
        return;
      }
      const resolvedAuthors = authors.filter((author) => author !== null);
      if (resolvedAuthors.length > 0) {
        setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, resolvedAuthors));
      }
    });
    return () => {
      disposed = true;
    };
  }, [
    activeGameRooms,
    api,
    knownAuthorsByPubkey,
    localProfile?.pubkey,
    setKnownAuthorsByPubkey,
    shellChromeState.activePrimarySection,
    state.syncStatus.local_author_pubkey,
  ]);

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
    adultContentEnabled,
  });

  // #858: 表示設定 OFF の間にゲート対象となる添付 hash(引用 snapshot 含む)。
  // effects 側で表示済み object URL の破棄と再取得抑止に使う。
  const gatedAdultMediaHashes = useMemo(() => {
    if (adultContentEnabled) {
      return [] as string[];
    }
    const hashes = new Set<string>();
    for (const post of [
      ...activeTimeline,
      ...activePublicTimeline,
      ...profileTimeline,
      ...selectedAuthorTimeline,
      ...thread,
    ]) {
      if (!isAdultLabeledPost(post)) {
        continue;
      }
      for (const attachment of post.attachments) {
        hashes.add(attachment.hash);
      }
      for (const attachment of post.repost_of?.attachments ?? []) {
        hashes.add(attachment.hash);
      }
    }
    return [...hashes];
  }, [
    activePublicTimeline,
    activeTimeline,
    adultContentEnabled,
    profileTimeline,
    selectedAuthorTimeline,
    thread,
  ]);

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
      setPendingTimelineNextCursorByKey(removeRecordEntry(key));
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
        privateTimelineScope(
          activeWorkspaceScope(storeApi.getState().workspaceState).topicId === topic
            ? activeWorkspaceScope(storeApi.getState().workspaceState).channelId
            : null
        )
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
        setTimelinesByKey(updateRecordEntry(key, (prev) => mergeRefreshedVisiblePosts(
            prev ?? EMPTY_POSTS,
            pendingItems,
            preserveOlderPages
          )));
        setTimelineNextCursorByKey(setRecordEntry(key, currentState.pendingTimelineNextCursorByKey[key] ?? null));
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
      mode: 'apply' | 'buffer' = 'buffer',
      scopeChannelId?: string | null
    ) => {
      const requestState = storeApi.getState();
      const selectedChannelId =
        scopeChannelId === undefined
          ? activeWorkspaceScope(requestState.workspaceState).topicId === topic
            ? activeWorkspaceScope(requestState.workspaceState).channelId
            : null
          : scopeChannelId;
      const timelineScope = privateTimelineScope(selectedChannelId);
      const timelineKey = timelineScopeStorageKey(topic, timelineScope);
      const requestId = (loadTopicsRequestRef.current.get(timelineKey) ?? 0) + 1;
      loadTopicsRequestRef.current.set(timelineKey, requestId);

      const [
        timelineResult,
        publicTimelineResult,
        joinedChannelsResult,
        threadViewResult,
      ] = await Promise.allSettled([
        api.listTimeline(topic, null, VISIBLE_TIMELINE_LIMIT, timelineScope),
        selectedChannelId === null
          ? Promise.resolve(null)
          : api.listTimeline(topic, null, VISIBLE_TIMELINE_LIMIT, PUBLIC_TIMELINE_SCOPE),
        api.listJoinedPrivateChannels(topic),
        currentThread
          ? api.listThread(topic, currentThread, null, THREAD_TIMELINE_LIMIT)
          : Promise.resolve(null),
      ]);

      if (requestId !== loadTopicsRequestRef.current.get(timelineKey)) {
        return;
      }

      const firstCoreFailure = [
        timelineResult,
        publicTimelineResult,
        joinedChannelsResult,
        threadViewResult,
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
            setPendingTimelineSnapshotsByKey(setRecordEntry(timelineKey, normalizedTimelineItems));
            setPendingTimelineCountsByKey(setRecordEntry(timelineKey, pendingCount));
            setPendingTimelineNextCursorByKey(setRecordEntry(timelineKey, resolvedTimelineCursor));
          } else {
            setTimelinesByKey(updateRecordEntry(timelineKey, (prev) => mergeRefreshedVisiblePosts(
                prev ?? EMPTY_POSTS,
                normalizedTimelineItems,
                preserveTimelinePages
              )));
            setTimelineNextCursorByKey(setRecordEntry(timelineKey, resolvedTimelineCursor));
            clearPendingTimeline(timelineKey);
          }
        }

        if (publicTimelineResult.status === 'fulfilled' && publicTimelineResult.value) {
          const publicTimeline = publicTimelineResult.value;
          const publicTimelineKey = timelineScopeStorageKey(topic, PUBLIC_TIMELINE_SCOPE);
          const baselinePublicTimeline =
            currentState.timelinesByKey[publicTimelineKey] ?? EMPTY_POSTS;
          const preservePublicTimelinePages =
            mode === 'buffer' &&
            hasLoadedOlderAuthoritativePosts(baselinePublicTimeline, publicTimeline.items);
          const resolvedPublicTimelineCursor = preservePublicTimelinePages
            ? (currentState.timelineNextCursorByKey[publicTimelineKey] ?? null)
            : (publicTimeline.next_cursor ?? null);
          setTimelinesByKey(updateRecordEntry(publicTimelineKey, (prev) => mergeRefreshedVisiblePosts(
              prev ?? EMPTY_POSTS,
              publicTimeline.items,
              preservePublicTimelinePages
            )));
          setTimelineNextCursorByKey(
            setRecordEntry(publicTimelineKey, resolvedPublicTimelineCursor)
          );
        }

        if (joinedChannelsResult.status === 'fulfilled') {
          setJoinedChannelsByTopic(setRecordEntry(topic, joinedChannelsResult.value));
          setChannelPanelStateByTopic(setRecordEntry(topic, {
              status: 'ready',
              error: null,
            }));
        } else {
          setChannelPanelStateByTopic(setRecordEntry(topic, {
              status: 'error',
              error: messageFromError(
                joinedChannelsResult.reason,
                translate('common:errors.failedToLoadPrivateChannels')
              ),
            }));
        }

        if (currentThread) {
          if (threadViewResult.status === 'fulfilled') {
            const threadView = threadViewResult.value;
            const incomingThreadItems = threadView?.items ?? [];
            const currentThreadPosts = currentState.threadsById[currentThread] ?? EMPTY_POSTS;
            const preserveThreadPages =
              mode === 'buffer' &&
              hasLoadedOlderAuthoritativePosts(currentThreadPosts, incomingThreadItems);
            const resolvedThreadCursor = preserveThreadPages
              ? (currentState.threadNextCursorById[currentThread] ?? null)
              : (threadView?.next_cursor ?? null);
            setThreadsById((current) => ({
              ...current,
              [currentThread]: mergeRefreshedVisiblePosts(
                current[currentThread] ?? [],
                incomingThreadItems,
                preserveThreadPages
              ),
            }));
            setThreadNextCursorById(setRecordEntry(currentThread, resolvedThreadCursor));
          }
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
      setError,
      setChannelPanelStateByTopic,
      setJoinedChannelsByTopic,
      setPendingTimelineCountsByKey,
      setPendingTimelineNextCursorByKey,
      setPendingTimelineSnapshotsByKey,
      setThreadsById,
      setThreadNextCursorById,
      setTimelineNextCursorByKey,
      setTimelinesByKey,
      storeApi,
      translate,
    ]
  );

  const loadMoreTimeline = useCallback(
    async (topic: string, scopeChannelId?: string | null) => {
      const currentState = storeApi.getState();
      const selectedChannelId =
        scopeChannelId === undefined
          ? activeWorkspaceScope(currentState.workspaceState).topicId === topic
            ? activeWorkspaceScope(currentState.workspaceState).channelId
            : null
          : scopeChannelId;
      const timelineScope = privateTimelineScope(selectedChannelId);
      const timelineKey = timelineScopeStorageKey(topic, timelineScope);
      const cursor = currentState.timelineNextCursorByKey[timelineKey] ?? null;
      if (!cursor || currentState.timelineLoadingMoreByKey[timelineKey]) {
        return;
      }
      setTimelineLoadingMoreByKey(setRecordEntry(timelineKey, true));
      try {
        const timeline = await api.listTimeline(
          topic,
          cursor,
          VISIBLE_TIMELINE_LIMIT,
          timelineScope
        );
        startTransition(() => {
          setTimelinesByKey(updateRecordEntry(timelineKey, (prev) => mergeUniquePosts(prev ?? EMPTY_POSTS, timeline.items)));
          setTimelineNextCursorByKey(setRecordEntry(timelineKey, timeline.next_cursor ?? null));
        });
      } finally {
        setTimelineLoadingMoreByKey(setRecordEntry(timelineKey, false));
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
      setThreadLoadingMoreById(setRecordEntry(threadId, true));
      try {
        const threadView = await api.listThread(topic, threadId, cursor, THREAD_TIMELINE_LIMIT);
        startTransition(() => {
          setThreadsById((current) => ({
            ...current,
            [threadId]: mergeUniquePosts(current[threadId] ?? [], threadView.items),
          }));
          setThreadNextCursorById(setRecordEntry(threadId, threadView.next_cursor ?? null));
        });
      } finally {
        setThreadLoadingMoreById(setRecordEntry(threadId, false));
      }
    },
    [
      api,
      setThreadsById,
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

  const {
    loadShellSections,
    loadProfileSection,
    loadAuthorSection,
    loadMessagesSection,
    loadNotificationsSection,
    loadCommunityIndexCapability,
  } = useDesktopShellSectionLoaders({
    api,
    loadReactionCatalogData,
    storeApi,
    translate,
  });
  const runLoadTopics = useCallback(
    async (_currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      await refreshVisibleShellData(currentActiveTopic, currentThread, 'apply');
      await loadShellSections(currentActiveTopic);
    },
    [loadShellSections, refreshVisibleShellData]
  );

  const refreshConnectivityStatus = useConnectivityStatusRefresh(
    api,
    setSyncStatus,
    setCommunityNodeStatuses
  );

  const queuedLoadTopics = useQueuedLoadTopics(runLoadTopics);
  const loadTopics = useCallback(
    async (topics: string[], currentActiveTopic: string, currentThread: string | null) => {
      await queuedLoadTopics(topics, currentActiveTopic, currentThread);
      await refreshConnectivityStatus();
    },
    [queuedLoadTopics, refreshConnectivityStatus]
  );

  const refreshVisibleTimelineAfterPublish = useCallback(
    async (topic: string, currentThread: string | null, scopeChannelId?: string | null) => {
      await refreshVisibleShellData(topic, currentThread, 'apply', scopeChannelId);
    },
    [refreshVisibleShellData]
  );

  const refreshTimelineFeed = useCallback(
    async (topic: string, currentThread: string | null, scopeChannelId?: string | null) => {
      const timelineScope = privateTimelineScope(
        scopeChannelId === undefined
          ? activeWorkspaceScope(storeApi.getState().workspaceState).topicId === topic
            ? activeWorkspaceScope(storeApi.getState().workspaceState).channelId
            : null
          : scopeChannelId
      );
      if (applyPendingTimeline(topic, timelineScope)) {
        return;
      }
      await refreshVisibleShellData(topic, currentThread, 'apply', scopeChannelId);
    },
    [applyPendingTimeline, refreshVisibleShellData, storeApi]
  );

  useDesktopShellDataEffects({
    api,
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
    gatedAdultMediaHashes,
    remoteObjectUrlRef,
    draftPreviewUrlRef,
    directMessageDraftPreviewUrlRef,
    mediaFetchAttemptRef,
    visibleRefreshInFlightRef,
    visibleColumnIdsRef,
    loadTopics,
    loadProfileSection,
    loadAuthorSection,
    loadMessagesSection,
    loadNotificationsSection,
    loadCommunityIndexCapability,
    refreshVisibleShellData,
    refreshConnectivityStatus,
    setNotificationStatus,
    setCommunityNodeStatuses,
    setSyncStatus,
    setLocalProfile,
    setProfileDraft,
    setNotifications,
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
    loadNotificationsSection,
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
