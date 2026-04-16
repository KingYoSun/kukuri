import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  type MutableRefObject,
} from 'react';

import type {
  AttachmentView,
  DesktopApi,
  DirectMessageMessageView,
  GameRoomView,
  JoinedPrivateChannelView,
  NotificationView,
  PostView,
} from '@/lib/api';

import {
  buildImageDraftItem,
  buildVideoDraftItem,
  createObjectUrlFromPayload,
  logMediaDebug,
  selectPrimaryImage,
  selectPrimaryImageAttachment,
  selectVideoManifest,
  selectVideoManifestAttachment,
  selectVideoPoster,
  selectVideoPosterAttachment,
} from '@/shell/media';
import {
  activeTimelineStorageKey,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  REFRESH_INTERVAL_MS,
  STATUS_REFRESH_INTERVAL_MS,
  timelineScopeStorageKey,
  type DraftMediaItem,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  useDesktopShellStoreApi,
} from '@/shell/store';
import {
  authorViewFromDirectMessageConversation,
  communityNodesToEditorValue,
  createGameEditorDraft,
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
const VISIBLE_TIMELINE_LIMIT = 20;
const THREAD_TIMELINE_LIMIT = 30;
type LoadTopicsArgs = readonly [string[], string, string | null];
type LoadTopicsWaiter = {
  resolve: () => void;
  reject: (error: unknown) => void;
};

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
  const loadTopicsInFlightRef = useRef(false);
  const queuedLoadTopicsArgsRef = useRef<LoadTopicsArgs | null>(null);
  const loadTopicsWaitersRef = useRef<LoadTopicsWaiter[]>([]);
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

  const previewableMediaAttachments = useMemo(() => {
    const attachments = new Map<string, AttachmentView>();
    const tryAddAttachment = (attachment: AttachmentView | null) => {
      if (!attachment) {
        return;
      }
      const hash = attachment.hash.trim();
      const mime = attachment.mime.trim();
      if (!hash || !mime) {
        logMediaDebug('warn', 'remote media metadata skipped', {
          hash: attachment.hash || null,
          mime: attachment.mime || null,
          role: attachment.role,
          status: attachment.status,
        });
        return;
      }
      attachments.set(hash, {
        ...attachment,
        hash,
        mime,
      });
    };

    for (const post of [
      ...activeTimeline,
      ...activePublicTimeline,
      ...profileTimeline,
      ...selectedAuthorTimeline,
      ...thread,
    ]) {
      if (post.author_picture_asset) {
        tryAddAttachment({
          hash: post.author_picture_asset.hash,
          mime: post.author_picture_asset.mime,
          bytes: post.author_picture_asset.bytes,
          role: post.author_picture_asset.role,
          status: 'Available',
        });
      }
      for (const attachment of [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ]) {
        tryAddAttachment(attachment);
      }
      for (const reaction of post.reaction_summary ?? []) {
        if (!reaction.custom_asset) {
          continue;
        }
        tryAddAttachment({
          hash: reaction.custom_asset.blob_hash,
          mime: reaction.custom_asset.mime,
          bytes: reaction.custom_asset.bytes,
          role: 'image_original',
          status: 'Available',
        });
      }
    }

    for (const message of selectedDirectMessageTimeline) {
      for (const attachment of [
        selectPrimaryImageAttachment(message.attachments),
        selectVideoPosterAttachment(message.attachments),
        selectVideoManifestAttachment(message.attachments),
      ]) {
        tryAddAttachment(attachment);
      }
    }

    for (const asset of [...ownedReactionAssets, ...bookmarkedReactionAssets]) {
      tryAddAttachment({
        hash: asset.blob_hash,
        mime: asset.mime,
        bytes: asset.bytes,
        role: 'image_original',
        status: 'Available',
      });
    }

    for (const reaction of recentReactions) {
      if (!reaction.custom_asset) {
        continue;
      }
      tryAddAttachment({
        hash: reaction.custom_asset.blob_hash,
        mime: reaction.custom_asset.mime,
        bytes: reaction.custom_asset.bytes,
        role: 'image_original',
        status: 'Available',
      });
    }

    for (const pictureAsset of [
      localProfile?.picture_asset ?? null,
      ...Object.values(knownAuthorsByPubkey).map((author) => author.picture_asset ?? null),
      ...notifications.map((notification) => notification.actor_picture_asset ?? null),
    ]) {
      tryAddAttachment(
        pictureAsset
          ? {
              hash: pictureAsset.hash,
              mime: pictureAsset.mime,
              bytes: pictureAsset.bytes,
              role: pictureAsset.role,
              status: 'Available',
            }
          : null
      );
    }

    return [...attachments.values()];
  }, [
    activePublicTimeline,
    activeTimeline,
    bookmarkedReactionAssets,
    knownAuthorsByPubkey,
    localProfile?.picture_asset,
    notifications,
    ownedReactionAssets,
    profileTimeline,
    recentReactions,
    selectedDirectMessageTimeline,
    selectedAuthorTimeline,
    thread,
  ]);

  const mergeUniquePosts = useCallback((current: PostView[], incoming: PostView[]) => {
    const seen = new Set(current.map((post) => post.object_id));
    return [...current, ...incoming.filter((post) => !seen.has(post.object_id))];
  }, []);

  const mergeLocalPosts = useCallback((current: PostView[], incoming: PostView[]) => {
    const authoritativeIds = new Set(incoming.map((post) => post.object_id));
    const localPosts = current.filter((post) => {
      if (!post.local_state) {
        return false;
      }
      const authoritativeId = post.server_object_id ?? post.object_id;
      return !authoritativeIds.has(authoritativeId);
    });
    const localObjectIds = new Set(localPosts.map((post) => post.object_id));
    return [...localPosts, ...incoming.filter((post) => !localObjectIds.has(post.object_id))];
  }, []);

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
      startTransition(() => {
        setTimelinesByKey((current) => ({
          ...current,
          [key]: mergeLocalPosts(current[key] ?? EMPTY_POSTS, pendingItems),
        }));
        setTimelineNextCursorByKey((current) => ({
          ...current,
          [key]: currentState.pendingTimelineNextCursorByKey[key] ?? null,
        }));
      });
      clearPendingTimeline(key);
      return true;
    },
    [clearPendingTimeline, mergeLocalPosts, setTimelineNextCursorByKey, setTimelinesByKey, storeApi]
  );

  const refreshVisibleShellData = useCallback(
    async (
      topic: string,
      currentThread: string | null,
      mode: 'apply' | 'buffer' = 'buffer'
    ) => {
      const requestId = loadTopicsRequestRef.current + 1;
      loadTopicsRequestRef.current = requestId;
      const currentState = storeApi.getState();
      const selectedChannelId = currentState.selectedChannelIdByTopic[topic] ?? null;
      const timelineScope = privateTimelineScope(selectedChannelId);
      const timelineKey = timelineScopeStorageKey(topic, timelineScope);

      try {
        const [timeline, publicTimeline, joinedChannels, threadView, status] = await Promise.all([
          api.listTimeline(topic, null, VISIBLE_TIMELINE_LIMIT, timelineScope),
          api.listTimeline(topic, null, VISIBLE_TIMELINE_LIMIT, PUBLIC_TIMELINE_SCOPE),
          api.listJoinedPrivateChannels(topic),
          currentThread
            ? api.listThread(topic, currentThread, null, THREAD_TIMELINE_LIMIT)
            : Promise.resolve(null),
          api.getSyncStatus(),
        ]);

        if (requestId !== loadTopicsRequestRef.current) {
          return;
        }

        startTransition(() => {
          const baselinePosts = currentState.timelinesByKey[timelineKey] ?? EMPTY_POSTS;
          const authoritativeIds = new Set(
            baselinePosts
              .filter((post) => !post.local_state)
              .map((post) => post.server_object_id ?? post.object_id)
          );
          const hasAuthoritativeBaseline = authoritativeIds.size > 0;
          const pendingCount = timeline.items.filter(
            (post) => !authoritativeIds.has(post.object_id)
          ).length;
          const shouldBuffer = mode === 'buffer' && hasAuthoritativeBaseline && pendingCount > 0;

          if (shouldBuffer) {
            setPendingTimelineSnapshotsByKey((current) => ({
              ...current,
              [timelineKey]: timeline.items,
            }));
            setPendingTimelineCountsByKey((current) => ({
              ...current,
              [timelineKey]: pendingCount,
            }));
            setPendingTimelineNextCursorByKey((current) => ({
              ...current,
              [timelineKey]: timeline.next_cursor ?? null,
            }));
          } else {
            setTimelinesByKey((current) => ({
              ...current,
              [timelineKey]: mergeLocalPosts(current[timelineKey] ?? EMPTY_POSTS, timeline.items),
            }));
            setTimelineNextCursorByKey((current) => ({
              ...current,
              [timelineKey]: timeline.next_cursor ?? null,
            }));
            clearPendingTimeline(timelineKey);
          }
          setPublicTimelinesByTopic((current) => ({
            ...current,
            [topic]: mergeLocalPosts(current[topic] ?? EMPTY_POSTS, publicTimeline.items),
          }));
          setPublicTimelineNextCursorByTopic((current) => ({
            ...current,
            [topic]: publicTimeline.next_cursor ?? null,
          }));
          setJoinedChannelsByTopic((current) => ({
            ...current,
            [topic]: joinedChannels,
          }));
          if (currentThread) {
            setThread((current) => mergeLocalPosts(current, threadView?.items ?? []));
            setThreadNextCursorById((current) => ({
              ...current,
              [currentThread]: threadView?.next_cursor ?? null,
            }));
          } else {
            setThread([]);
          }
          setSyncStatus(status);
          setError(null);
        });
      } catch (refreshError) {
        if (requestId !== loadTopicsRequestRef.current) {
          return;
        }
        setError(
          refreshError instanceof Error
            ? refreshError.message
            : translate('common:errors.failedToLoadTopic')
        );
      }
    },
    [
      api,
      mergeLocalPosts,
      clearPendingTimeline,
      loadTopicsRequestRef,
      setError,
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
      mergeUniquePosts,
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
      mergeUniquePosts,
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
                    setCommunityNodeInput(communityNodesToEditorValue(config));
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

  const drainLoadTopicsQueue = useCallback(
    async (initialArgs: LoadTopicsArgs) => {
      let nextArgs: LoadTopicsArgs | null = initialArgs;
      let lastError: unknown = null;

      while (nextArgs) {
        queuedLoadTopicsArgsRef.current = null;
        try {
          await runLoadTopics(...nextArgs);
          lastError = null;
        } catch (error) {
          lastError = error;
        }
        nextArgs = queuedLoadTopicsArgsRef.current;
      }

      loadTopicsInFlightRef.current = false;
      const waiters = loadTopicsWaitersRef.current;
      loadTopicsWaitersRef.current = [];
      for (const waiter of waiters) {
        if (lastError) {
          waiter.reject(lastError);
          continue;
        }
        waiter.resolve();
      }
    },
    [runLoadTopics]
  );

  const loadTopics = useCallback(
    (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      const args: LoadTopicsArgs = [[...currentTopics], currentActiveTopic, currentThread];
      return new Promise<void>((resolve, reject) => {
        loadTopicsWaitersRef.current.push({ resolve, reject });
        if (loadTopicsInFlightRef.current) {
          queuedLoadTopicsArgsRef.current = args;
          return;
        }
        loadTopicsInFlightRef.current = true;
        void drainLoadTopicsQueue(args);
      });
    },
    [drainLoadTopicsQueue]
  );

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

  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (
        disposed ||
        visibleRefreshInFlightRef.current ||
        (typeof document !== 'undefined' && document.visibilityState === 'hidden')
      ) {
        return;
      }
      visibleRefreshInFlightRef.current = true;
      try {
        await refreshVisibleShellData(activeTopic, selectedThread, 'buffer');
      } finally {
        visibleRefreshInFlightRef.current = false;
      }
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);
    const handleFocus = () => {
      void refresh();
    };
    const handleVisibility = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
        void refresh();
      }
    };
    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      disposed = true;
      visibleRefreshInFlightRef.current = false;
      window.clearInterval(intervalId);
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [activeTopic, refreshVisibleShellData, selectedThread]);

  useEffect(() => {
    let disposed = false;

    const refreshStatus = async () => {
      if (
        disposed ||
        (typeof document !== 'undefined' && document.visibilityState === 'hidden')
      ) {
        return;
      }
      try {
        const status = await api.getNotificationStatus();
        if (!disposed) {
          setNotificationStatus(status);
        }
      } catch {
        // best effort badge refresh
      }
    };

    void refreshStatus();
    const intervalId = window.setInterval(() => {
      void refreshStatus();
    }, STATUS_REFRESH_INTERVAL_MS);
    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [api, setNotificationStatus]);

  useEffect(() => {
    let disposed = false;
    void (async () => {
      try {
        const profile = await api.getMyProfile();
        if (disposed) {
          return;
        }
        setLocalProfile(profile);
        if (!storeApi.getState().profileDirty) {
          setProfileDraft(profileInputFromProfile(profile));
        }
      } catch {
        // best effort background bootstrap
      }
    })();
    return () => {
      disposed = true;
    };
  }, [api, setLocalProfile, setProfileDraft, storeApi]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'live') {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [activeTopic, loadTopics, selectedThread, shellChromeState.activePrimarySection, trackedTopics]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'game') {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [activeTopic, loadTopics, selectedThread, shellChromeState.activePrimarySection, trackedTopics]);

  useEffect(() => {
    if (
      shellChromeState.activePrimarySection !== 'timeline' ||
      shellChromeState.timelineView !== 'bookmarks'
    ) {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [
    activeTopic,
    loadTopics,
    selectedThread,
    shellChromeState.activePrimarySection,
    shellChromeState.timelineView,
    trackedTopics,
  ]);

  useEffect(() => {
    if (!shellChromeState.settingsOpen) {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [
    activeTopic,
    loadTopics,
    selectedThread,
    shellChromeState.activeSettingsSection,
    shellChromeState.settingsOpen,
    trackedTopics,
  ]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'profile') {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const profile = await api.getMyProfile();
        if (disposed) {
          return;
        }
        setLocalProfile(profile);
        if (!storeApi.getState().profileDirty) {
          setProfileDraft(profileInputFromProfile(profile));
        }
        const [timeline, following, followed, muted] = await Promise.all([
          api.listProfileTimeline(profile.pubkey, null, VISIBLE_TIMELINE_LIMIT),
          api.listSocialConnections('following'),
          api.listSocialConnections('followed'),
          api.listSocialConnections('muted'),
        ]);
        if (disposed) {
          return;
        }
        startTransition(() => {
          setProfileTimeline(timeline.items);
          setProfileTimelineNextCursor(timeline.next_cursor ?? null);
          setProfilePanelState({ status: 'ready', error: null });
          setProfileError(null);
          setSocialConnections({
            following,
            followed,
            muted,
          });
          setKnownAuthorsByPubkey((current) =>
            mergeKnownAuthors(current, [...following, ...followed, ...muted])
          );
          setSocialConnectionsPanelState({ status: 'ready', error: null });
        });
      } catch (error) {
        if (!disposed) {
          const message = messageFromError(
            error,
            translate('common:errors.failedToLoadProfile')
          );
          setProfileError(message);
          setProfilePanelState({ status: 'error', error: message });
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, [
    api,
    setKnownAuthorsByPubkey,
    setLocalProfile,
    setProfileDraft,
    setProfileError,
    setProfilePanelState,
    setProfileTimeline,
    setProfileTimelineNextCursor,
    setSocialConnections,
    setSocialConnectionsPanelState,
    shellChromeState.activePrimarySection,
    storeApi,
    translate,
  ]);

  useEffect(() => {
    if (!selectedAuthorPubkey) {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const [author, timeline] = await Promise.all([
          api.getAuthorSocialView(selectedAuthorPubkey),
          api.listProfileTimeline(selectedAuthorPubkey, null, VISIBLE_TIMELINE_LIMIT),
        ]);
        if (disposed) {
          return;
        }
        startTransition(() => {
          setSelectedAuthor(author);
          setSelectedAuthorTimeline(timeline.items);
          setSelectedAuthorTimelineNextCursor(timeline.next_cursor ?? null);
          setAuthorError(null);
          if (author) {
            setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [author]));
          }
        });
      } catch (error) {
        if (!disposed) {
          setAuthorError(
            messageFromError(error, translate('common:errors.failedToLoadAuthor'))
          );
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, [
    api,
    selectedAuthorPubkey,
    setAuthorError,
    setKnownAuthorsByPubkey,
    setSelectedAuthor,
    setSelectedAuthorTimeline,
    setSelectedAuthorTimelineNextCursor,
    translate,
  ]);

  useEffect(() => {
    if (
      shellChromeState.activePrimarySection !== 'messages' &&
      !storeApi.getState().directMessagePaneOpen
    ) {
      return;
    }
    let disposed = false;
    const refresh = async () => {
      if (
        disposed ||
        (typeof document !== 'undefined' && document.visibilityState === 'hidden')
      ) {
        return;
      }
      try {
        const directMessages = await api.listDirectMessages();
        if (disposed) {
          return;
        }
        setDirectMessages(directMessages);
        setKnownAuthorsByPubkey((current) =>
          mergeKnownAuthors(current, directMessages.map(authorViewFromDirectMessageConversation))
        );
        const selectedPeerPubkey = storeApi.getState().selectedDirectMessagePeerPubkey;
        if (!selectedPeerPubkey) {
          setDirectMessageError(null);
          return;
        }
        const [timelineResult, statusResult] = await Promise.allSettled([
          api.listDirectMessageMessages(selectedPeerPubkey, null, VISIBLE_TIMELINE_LIMIT),
          api.getDirectMessageStatus(selectedPeerPubkey),
        ]);
        if (disposed) {
          return;
        }
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
      } catch (error) {
        if (!disposed) {
          setDirectMessageError(messageFromError(error, 'failed to load direct messages'));
        }
      }
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);
    const handleFocus = () => {
      void refresh();
    };
    const handleVisibility = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
        void refresh();
      }
    };
    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleVisibility);
    return () => {
      disposed = true;
      window.clearInterval(intervalId);
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [
    api,
    setDirectMessageError,
    setDirectMessages,
    setDirectMessageStatusByPeer,
    setDirectMessageTimelineByPeer,
    setDirectMessageTimelineNextCursorByPeer,
    setKnownAuthorsByPubkey,
    shellChromeState.activePrimarySection,
    storeApi,
  ]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'notifications') {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const [status, notificationItems] = await Promise.all([
          api.getNotificationStatus(),
          api.listNotifications(),
        ]);
        if (disposed) {
          return;
        }
        let nextNotifications = notificationItems;
        let nextStatus = status;
        if (notificationItems.some((notification) => !notification.read_at)) {
          try {
            nextStatus = await api.markAllNotificationsRead();
            const readAt = Date.now();
            nextNotifications = notificationItems.map((notification) =>
              notification.read_at ? notification : { ...notification, read_at: readAt }
            );
            if (!disposed) {
              setNotificationAutoReadError(null);
            }
          } catch (notificationReadError) {
            if (!disposed) {
              setNotificationAutoReadError(
                messageFromError(
                  notificationReadError,
                  translate('shell:notifications.errors.failedAutoRead')
                )
              );
            }
          }
        }
        if (disposed) {
          return;
        }
        startTransition(() => {
          setNotificationStatus(nextStatus);
          setNotifications(nextNotifications);
          setNotificationPanelState({ status: 'ready', error: null });
        });
      } catch (error) {
        if (!disposed) {
          setNotificationPanelState({
            status: 'error',
            error: messageFromError(error, translate('shell:notifications.errors.failedToLoad')),
          });
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, [
    api,
    setNotificationAutoReadError,
    setNotificationPanelState,
    setNotifications,
    setNotificationStatus,
    shellChromeState.activePrimarySection,
    translate,
  ]);

  useEffect(() => {
    const remoteObjectUrls = remoteObjectUrlRef.current;
    const draftPreviewUrls = draftPreviewUrlRef.current;
    const directMessageDraftPreviewUrls = directMessageDraftPreviewUrlRef.current;

    return () => {
      for (const url of remoteObjectUrls.values()) {
        URL.revokeObjectURL(url);
      }
      remoteObjectUrls.clear();
      for (const url of draftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      draftPreviewUrls.clear();
      for (const url of directMessageDraftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      directMessageDraftPreviewUrls.clear();
    };
  }, [directMessageDraftPreviewUrlRef, draftPreviewUrlRef, remoteObjectUrlRef]);

  useEffect(() => {
    setGameDrafts((current) => {
      let changed = false;
      const next = { ...current };
      for (const room of activeGameRooms) {
        if (!next[room.room_id]) {
          next[room.room_id] = createGameEditorDraft(room);
          changed = true;
        }
      }
      return changed ? next : current;
    });
  }, [activeGameRooms, setGameDrafts]);

  useEffect(() => {
    if (!selectedPrivateChannelId) {
      return;
    }
    const selectedStillJoined = activeJoinedChannels.some(
      (channel) => channel.channel_id === selectedPrivateChannelId
    );
    if (selectedStillJoined) {
      return;
    }
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [activeTopic]: null,
    }));
    setComposeChannelByTopic((current) =>
      current[activeTopic]?.kind === 'private_channel' &&
      current[activeTopic].channel_id === selectedPrivateChannelId
        ? {
            ...current,
            [activeTopic]: PUBLIC_CHANNEL_REF,
          }
        : current
    );
    setTimelineScopeByTopic((current) =>
      current[activeTopic]?.kind === 'channel' &&
      current[activeTopic].channel_id === selectedPrivateChannelId
        ? {
            ...current,
            [activeTopic]: PUBLIC_TIMELINE_SCOPE,
          }
        : current
    );
  }, [
    activeJoinedChannels,
    activeTopic,
    selectedPrivateChannelId,
    setComposeChannelByTopic,
    setSelectedChannelIdByTopic,
    setTimelineScopeByTopic,
  ]);

  useEffect(() => {
    let disposed = false;

    for (const attachment of previewableMediaAttachments) {
      if (typeof mediaObjectUrls[attachment.hash] === 'string') {
        continue;
      }

      const nextAttempt = (mediaFetchAttemptRef.current.get(attachment.hash) ?? 0) + 1;
      mediaFetchAttemptRef.current.set(attachment.hash, nextAttempt);
      logMediaDebug('info', 'remote media fetch start', {
        attempt: nextAttempt,
        hash: attachment.hash,
        mime: attachment.mime,
        role: attachment.role,
        status: attachment.status,
      });

      void api
        .getBlobMediaPayload(attachment.hash, attachment.mime)
        .then((payload) => {
          const nextUrl = payload ? createObjectUrlFromPayload(payload) : null;
          if (disposed) {
            if (nextUrl) {
              URL.revokeObjectURL(nextUrl);
            }
            return;
          }
          if (!nextUrl) {
            logMediaDebug('warn', 'remote media fetch missing', {
              attempt: nextAttempt,
              hash: attachment.hash,
              mime: attachment.mime,
              role: attachment.role,
              status: attachment.status,
            });
            return;
          }

          logMediaDebug('info', 'remote media fetch hit', {
            attempt: nextAttempt,
            bytes_base64_length: payload?.bytes_base64.length ?? 0,
            hash: attachment.hash,
            mime: attachment.mime,
            object_url: nextUrl,
            role: attachment.role,
            status: attachment.status,
          });

          setMediaObjectUrls((current) => {
            if (current[attachment.hash] !== undefined) {
              URL.revokeObjectURL(nextUrl);
              return current;
            }
            remoteObjectUrlRef.current.set(attachment.hash, nextUrl);
            return {
              ...current,
              [attachment.hash]: nextUrl,
            };
          });
        })
        .catch((fetchError: unknown) => {
          if (disposed) {
            return;
          }
          logMediaDebug('warn', 'remote media fetch error', {
            attempt: nextAttempt,
            error: fetchError instanceof Error ? fetchError.message : 'unknown error',
            hash: attachment.hash,
            mime: attachment.mime,
            role: attachment.role,
            status: attachment.status,
          });
        });
    }

    return () => {
      disposed = true;
    };
  }, [
    api,
    mediaFetchAttemptRef,
    mediaObjectUrls,
    previewableMediaAttachments,
    remoteObjectUrlRef,
    setMediaObjectUrls,
  ]);

  const nextDraftId = useCallback((): string => {
    draftSequenceRef.current += 1;
    return `draft-${draftSequenceRef.current}`;
  }, [draftSequenceRef]);

  const rememberDraftPreview = useCallback(
    (item: DraftMediaItem) => {
      draftPreviewUrlRef.current.set(item.id, item.preview_url);
    },
    [draftPreviewUrlRef]
  );

  const releaseDraftPreview = useCallback(
    (itemId: string) => {
      const previewUrl = draftPreviewUrlRef.current.get(itemId);
      if (!previewUrl) {
        return;
      }
      URL.revokeObjectURL(previewUrl);
      draftPreviewUrlRef.current.delete(itemId);
    },
    [draftPreviewUrlRef]
  );

  const releaseAllDraftPreviews = useCallback(() => {
    for (const [itemId, previewUrl] of draftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      draftPreviewUrlRef.current.delete(itemId);
    }
  }, [draftPreviewUrlRef]);

  const rememberDirectMessageDraftPreview = useCallback(
    (item: DraftMediaItem) => {
      directMessageDraftPreviewUrlRef.current.set(item.id, item.preview_url);
    },
    [directMessageDraftPreviewUrlRef]
  );

  const releaseDirectMessageDraftPreview = useCallback(
    (itemId: string) => {
      const previewUrl = directMessageDraftPreviewUrlRef.current.get(itemId);
      if (!previewUrl) {
        return;
      }
      URL.revokeObjectURL(previewUrl);
      directMessageDraftPreviewUrlRef.current.delete(itemId);
    },
    [directMessageDraftPreviewUrlRef]
  );

  const releaseAllDirectMessageDraftPreviews = useCallback(() => {
    for (const [itemId, previewUrl] of directMessageDraftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      directMessageDraftPreviewUrlRef.current.delete(itemId);
    }
  }, [directMessageDraftPreviewUrlRef]);

  const buildImageItem = useCallback(
    async (file: File) => {
      return await buildImageDraftItem(file, nextDraftId);
    },
    [nextDraftId]
  );

  const buildVideoItem = useCallback(
    async (file: File) => {
      return await buildVideoDraftItem(file, nextDraftId);
    },
    [nextDraftId]
  );

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
    buildImageDraftItem: buildImageItem,
    buildVideoDraftItem: buildVideoItem,
  };
}
