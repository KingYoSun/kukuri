import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  type MutableRefObject,
} from 'react';

import type {
  AttachmentView,
  DesktopApi,
  DirectMessageMessageView,
  GameRoomView,
  JoinedPrivateChannelView,
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
  DEFAULT_SOCIAL_CONNECTIONS,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  REFRESH_INTERVAL_MS,
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
    directMessagePaneOpen,
    selectedDirectMessagePeerPubkey,
    profileTimeline,
    selectedAuthorTimeline,
    thread,
    ownedReactionAssets,
    bookmarkedReactionAssets,
  } = state;
  const selectedDirectMessageTimeline =
    state.directMessageTimelineByPeer[state.selectedDirectMessagePeerPubkey ?? ''] ??
    EMPTY_DIRECT_MESSAGE_TIMELINE;
  const activePublicTimeline = state.publicTimelinesByTopic[activeTopic] ?? EMPTY_POSTS;
  const activeTimeline = state.timelinesByTopic[activeTopic] ?? EMPTY_POSTS;
  const activeGameRooms = gameRoomsByTopic[activeTopic] ?? EMPTY_GAME_ROOMS;
  const activeJoinedChannels = joinedChannelsByTopic[activeTopic] ?? EMPTY_JOINED_CHANNELS;
  const selectedPrivateChannelId = selectedChannelIdByTopic[activeTopic] ?? null;
  const selectedAuthorPubkey = state.selectedAuthorPubkey;

  const setTimelinesByTopic = useDesktopShellFieldSetter('timelinesByTopic');
  const setPublicTimelinesByTopic = useDesktopShellFieldSetter('publicTimelinesByTopic');
  const setLiveSessionsByTopic = useDesktopShellFieldSetter('liveSessionsByTopic');
  const setGameRoomsByTopic = useDesktopShellFieldSetter('gameRoomsByTopic');
  const setJoinedChannelsByTopic = useDesktopShellFieldSetter('joinedChannelsByTopic');
  const setSelectedChannelIdByTopic = useDesktopShellFieldSetter('selectedChannelIdByTopic');
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setThread = useDesktopShellFieldSetter('thread');
  const setLocalPeerTicket = useDesktopShellFieldSetter('localPeerTicket');
  const setDiscoveryConfig = useDesktopShellFieldSetter('discoveryConfig');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setCommunityNodeConfig = useDesktopShellFieldSetter('communityNodeConfig');
  const setCommunityNodeStatuses = useDesktopShellFieldSetter('communityNodeStatuses');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setMediaObjectUrls = useDesktopShellFieldSetter('mediaObjectUrls');
  const setSyncStatus = useDesktopShellFieldSetter('syncStatus');
  const setLocalProfile = useDesktopShellFieldSetter('localProfile');
  const setProfileTimeline = useDesktopShellFieldSetter('profileTimeline');
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
  const setAuthorError = useDesktopShellFieldSetter('authorError');
  const setSelectedDirectMessagePeerPubkey = useDesktopShellFieldSetter(
    'selectedDirectMessagePeerPubkey'
  );
  const setDirectMessages = useDesktopShellFieldSetter('directMessages');
  const setDirectMessageTimelineByPeer = useDesktopShellFieldSetter('directMessageTimelineByPeer');
  const setDirectMessageStatusByPeer = useDesktopShellFieldSetter('directMessageStatusByPeer');
  const setDirectMessageError = useDesktopShellFieldSetter('directMessageError');
  const setLivePanelStateByTopic = useDesktopShellFieldSetter('livePanelStateByTopic');
  const setChannelPanelStateByTopic = useDesktopShellFieldSetter('channelPanelStateByTopic');
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
      for (const attachment of [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ]) {
        tryAddAttachment(attachment);
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

    for (const pictureAsset of [
      localProfile?.picture_asset ?? null,
      ...Object.values(knownAuthorsByPubkey).map((author) => author.picture_asset ?? null),
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
    ownedReactionAssets,
    profileTimeline,
    selectedDirectMessageTimeline,
    selectedAuthorTimeline,
    thread,
  ]);

  const loadTopics = useCallback(
    async (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
      const requestId = loadTopicsRequestRef.current + 1;
      loadTopicsRequestRef.current = requestId;
      const currentState = storeApi.getState();
      const currentSelectedChannelIdByTopic = currentState.selectedChannelIdByTopic;
      const currentSelectedAuthorPubkey = currentState.selectedAuthorPubkey;
      const currentDirectMessagePaneOpen = currentState.directMessagePaneOpen;
      const currentSelectedDirectMessagePeerPubkey = currentState.selectedDirectMessagePeerPubkey;
      const currentDiscoveryEditorDirty = currentState.discoveryEditorDirty;
      const currentCommunityNodeEditorDirty = currentState.communityNodeEditorDirty;
      const currentProfileDirty = currentState.profileDirty;

      try {
        const [
          timelineViews,
          publicTimelineViews,
          liveViewsResult,
          gameViewsResult,
          joinedChannelViewsResult,
          threadView,
          directMessagesView,
          status,
        ] = await Promise.all([
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              timeline: await api.listTimeline(
                topic,
                null,
                50,
                privateTimelineScope(currentSelectedChannelIdByTopic[topic] ?? null)
              ),
            }))
          ),
          Promise.all(
            currentTopics.map(async (topic) => ({
              topic,
              timeline: await api.listTimeline(topic, null, 50, PUBLIC_TIMELINE_SCOPE),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              sessions: await api.listLiveSessions(
                topic,
                privateTimelineScope(currentSelectedChannelIdByTopic[topic] ?? null)
              ),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              rooms: await api.listGameRooms(
                topic,
                privateTimelineScope(currentSelectedChannelIdByTopic[topic] ?? null)
              ),
            }))
          ),
          Promise.allSettled(
            currentTopics.map(async (topic) => ({
              topic,
              channels: await api.listJoinedPrivateChannels(topic),
            }))
          ),
          currentThread
            ? api.listThread(currentActiveTopic, currentThread, null, 50)
            : Promise.resolve(null),
          api.listDirectMessages(),
          api.getSyncStatus(),
        ]);
        const [
          discoveryResult,
          communityConfigResult,
          communityStatusesResult,
          ticketResult,
          profileResult,
          authorViewResult,
          profileTimelineResult,
          authorTimelineResult,
          directMessageTimelineResult,
          directMessageStatusResult,
          ownedReactionAssetsResult,
          bookmarkedReactionAssetsResult,
          bookmarkedPostsResult,
          recentReactionsResult,
          followingConnectionsResult,
          followedConnectionsResult,
          mutedConnectionsResult,
        ] = await Promise.allSettled([
          api.getDiscoveryConfig(),
          api.getCommunityNodeConfig(),
          api.getCommunityNodeStatuses(),
          api.getLocalPeerTicket(),
          api.getMyProfile(),
          currentSelectedAuthorPubkey
            ? api.getAuthorSocialView(currentSelectedAuthorPubkey)
            : Promise.resolve(null),
          api.listProfileTimeline(status.local_author_pubkey, null, 50),
          currentSelectedAuthorPubkey
            ? api.listProfileTimeline(currentSelectedAuthorPubkey, null, 50)
            : Promise.resolve(null),
          currentDirectMessagePaneOpen && currentSelectedDirectMessagePeerPubkey
            ? api.listDirectMessageMessages(currentSelectedDirectMessagePeerPubkey, null, 100)
            : Promise.resolve(null),
          currentDirectMessagePaneOpen && currentSelectedDirectMessagePeerPubkey
            ? api.getDirectMessageStatus(currentSelectedDirectMessagePeerPubkey)
            : Promise.resolve(null),
          api.listMyCustomReactionAssets(),
          api.listBookmarkedCustomReactions(),
          api.listBookmarkedPosts(),
          api.listRecentReactions(8),
          api.listSocialConnections('following'),
          api.listSocialConnections('followed'),
          api.listSocialConnections('muted'),
        ]);
        if (requestId !== loadTopicsRequestRef.current) {
          return;
        }

        startTransition(() => {
          setTimelinesByTopic(
            Object.fromEntries(timelineViews.map(({ topic, timeline }) => [topic, timeline.items]))
          );
          setPublicTimelinesByTopic(
            Object.fromEntries(
              publicTimelineViews.map(({ topic, timeline }) => [topic, timeline.items])
            )
          );
          setLiveSessionsByTopic((current) => {
            const next = { ...current };
            for (const result of liveViewsResult) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = result.value.sessions;
              }
            }
            return next;
          });
          setGameRoomsByTopic((current) => {
            const next = { ...current };
            for (const result of gameViewsResult) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = result.value.rooms;
              }
            }
            return next;
          });
          setJoinedChannelsByTopic((current) => {
            const next = { ...current };
            for (const result of joinedChannelViewsResult) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = result.value.channels;
              }
            }
            return next;
          });
          setLivePanelStateByTopic((current) => {
            const next = { ...current };
            for (const [index, result] of liveViewsResult.entries()) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = {
                  status: 'ready',
                  error: null,
                };
              } else {
                next[currentTopics[index]] = {
                  status: 'error',
                  error: messageFromError(
                    result.reason,
                    translate('common:errors.failedToLoadLiveSessions')
                  ),
                };
              }
            }
            return next;
          });
          setGamePanelStateByTopic((current) => {
            const next = { ...current };
            for (const [index, result] of gameViewsResult.entries()) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = {
                  status: 'ready',
                  error: null,
                };
              } else {
                next[currentTopics[index]] = {
                  status: 'error',
                  error: messageFromError(
                    result.reason,
                    translate('common:errors.failedToLoadGameRooms')
                  ),
                };
              }
            }
            return next;
          });
          setChannelPanelStateByTopic((current) => {
            const next = { ...current };
            for (const [index, result] of joinedChannelViewsResult.entries()) {
              if (result.status === 'fulfilled') {
                next[result.value.topic] = {
                  status: 'ready',
                  error: null,
                };
              } else {
                next[currentTopics[index]] = {
                  status: 'error',
                  error: messageFromError(
                    result.reason,
                    translate('common:errors.failedToLoadPrivateChannels')
                  ),
                };
              }
            }
            return next;
          });
          setDirectMessages(directMessagesView);
          setKnownAuthorsByPubkey((current) =>
            mergeKnownAuthors(current, directMessagesView.map(authorViewFromDirectMessageConversation))
          );
          setSyncStatus(status);
          if (discoveryResult.status === 'fulfilled') {
            setDiscoveryConfig(discoveryResult.value);
            if (!currentDiscoveryEditorDirty) {
              setDiscoverySeedInput(seedPeersToEditorValue(discoveryResult.value));
            }
          }
          if (communityConfigResult.status === 'fulfilled') {
            setCommunityNodeConfig(communityConfigResult.value);
            if (!currentCommunityNodeEditorDirty) {
              setCommunityNodeInput(communityNodesToEditorValue(communityConfigResult.value));
            }
          }
          if (communityStatusesResult.status === 'fulfilled') {
            setCommunityNodeStatuses((current) =>
              mergeCommunityNodeStatuses(current, communityStatusesResult.value)
            );
          }
          if (ticketResult.status === 'fulfilled') {
            setLocalPeerTicket(ticketResult.value);
          }
          if (ownedReactionAssetsResult.status === 'fulfilled') {
            setOwnedReactionAssets(ownedReactionAssetsResult.value);
          }
          if (bookmarkedReactionAssetsResult.status === 'fulfilled') {
            setBookmarkedReactionAssets(bookmarkedReactionAssetsResult.value);
          }
          if (bookmarkedPostsResult.status === 'fulfilled') {
            setBookmarkedPosts(bookmarkedPostsResult.value);
          }
          if (recentReactionsResult.status === 'fulfilled') {
            setRecentReactions(recentReactionsResult.value);
          }
          if (
            followingConnectionsResult.status === 'fulfilled' &&
            followedConnectionsResult.status === 'fulfilled' &&
            mutedConnectionsResult.status === 'fulfilled'
          ) {
            setSocialConnections({
              following: followingConnectionsResult.value,
              followed: followedConnectionsResult.value,
              muted: mutedConnectionsResult.value,
            });
            setKnownAuthorsByPubkey((current) =>
              mergeKnownAuthors(current, [
                ...followingConnectionsResult.value,
                ...followedConnectionsResult.value,
                ...mutedConnectionsResult.value,
              ])
            );
            setSocialConnectionsPanelState({
              status: 'ready',
              error: null,
            });
          } else {
            setSocialConnections(DEFAULT_SOCIAL_CONNECTIONS);
            setSocialConnectionsPanelState({
              status: 'error',
              error:
                followingConnectionsResult.status === 'rejected'
                  ? messageFromError(
                      followingConnectionsResult.reason,
                      translate('common:errors.failedToLoadSocialConnections')
                    )
                  : followedConnectionsResult.status === 'rejected'
                    ? messageFromError(
                        followedConnectionsResult.reason,
                        translate('common:errors.failedToLoadSocialConnections')
                      )
                    : mutedConnectionsResult.status === 'rejected'
                      ? messageFromError(
                          mutedConnectionsResult.reason,
                          translate('common:errors.failedToLoadSocialConnections')
                        )
                      : null,
            });
          }
          setReactionPanelState({
            status:
              ownedReactionAssetsResult.status === 'fulfilled' &&
              bookmarkedReactionAssetsResult.status === 'fulfilled' &&
              recentReactionsResult.status === 'fulfilled'
                ? 'ready'
                : 'error',
            error:
              ownedReactionAssetsResult.status === 'rejected'
                ? messageFromError(
                    ownedReactionAssetsResult.reason,
                    translate('common:errors.failedToLoadSettings')
                  )
                : bookmarkedReactionAssetsResult.status === 'rejected'
                  ? messageFromError(
                      bookmarkedReactionAssetsResult.reason,
                      translate('common:errors.failedToLoadSettings')
                    )
                  : recentReactionsResult.status === 'rejected'
                    ? messageFromError(
                        recentReactionsResult.reason,
                        translate('common:errors.failedToLoadSettings')
                      )
                    : null,
          });
          if (profileResult.status === 'fulfilled') {
            setLocalProfile(profileResult.value);
            if (!currentProfileDirty) {
              setProfileDraft(profileInputFromProfile(profileResult.value));
            }
            if (profileTimelineResult.status === 'fulfilled') {
              setProfileTimeline(profileTimelineResult.value.items);
              setProfileError(null);
              setProfilePanelState({
                status: 'ready',
                error: null,
              });
            } else {
              const nextProfileError = messageFromError(
                profileTimelineResult.reason,
                translate('common:errors.failedToLoadProfile')
              );
              setProfileTimeline([]);
              setProfileError(nextProfileError);
              setProfilePanelState({
                status: 'error',
                error: nextProfileError,
              });
            }
          } else {
            const nextProfileError = messageFromError(
              profileResult.reason,
              translate('common:errors.failedToLoadProfile')
            );
            setProfileTimeline([]);
            setProfileError(nextProfileError);
            setProfilePanelState({
              status: 'error',
              error: nextProfileError,
            });
          }
          if (!currentSelectedAuthorPubkey) {
            setSelectedAuthor(null);
            setSelectedAuthorTimeline([]);
            setAuthorError(null);
          } else if (
            authorViewResult.status === 'fulfilled' &&
            authorTimelineResult.status === 'fulfilled'
          ) {
            setSelectedAuthor(authorViewResult.value);
            setSelectedAuthorTimeline(authorTimelineResult.value?.items ?? []);
            setAuthorError(null);
            if (authorViewResult.value) {
              setKnownAuthorsByPubkey((current) =>
                mergeKnownAuthors(current, [authorViewResult.value])
              );
            }
          } else {
            setSelectedAuthorTimeline([]);
            setAuthorError(
              messageFromError(
                authorViewResult.status === 'rejected'
                  ? authorViewResult.reason
                  : authorTimelineResult.status === 'rejected'
                    ? authorTimelineResult.reason
                    : null,
                translate('common:errors.failedToLoadAuthor')
              )
            );
          }
          if (!currentDirectMessagePaneOpen) {
            setSelectedDirectMessagePeerPubkey(null);
            setDirectMessageError(null);
          } else if (!currentSelectedDirectMessagePeerPubkey) {
            setDirectMessageError(null);
          } else {
            if (directMessageTimelineResult.status === 'fulfilled') {
              setDirectMessageTimelineByPeer((current) => ({
                ...current,
                [currentSelectedDirectMessagePeerPubkey]: directMessageTimelineResult.value?.items ?? [],
              }));
            }
            if (directMessageStatusResult.status === 'fulfilled') {
              setDirectMessageStatusByPeer((current) => ({
                ...current,
                [currentSelectedDirectMessagePeerPubkey]: directMessageStatusResult.value!,
              }));
            }
            if (
              directMessageTimelineResult.status === 'fulfilled' &&
              directMessageStatusResult.status === 'fulfilled'
            ) {
              setDirectMessageError(null);
            } else {
              setDirectMessageError(
                messageFromError(
                  directMessageTimelineResult.status === 'rejected'
                    ? directMessageTimelineResult.reason
                    : directMessageStatusResult.status === 'rejected'
                      ? directMessageStatusResult.reason
                      : null,
                  'failed to load direct messages'
                )
              );
            }
          }
          if (threadView) {
            setThread(threadView.items);
          } else if (!currentThread) {
            setThread([]);
          }
          setError(null);
        });
      } catch (loadError) {
        if (requestId !== loadTopicsRequestRef.current) {
          return;
        }
        setError(
          loadError instanceof Error
            ? loadError.message
            : translate('common:errors.failedToLoadTopic')
        );
      }
    },
    [
      api,
      loadTopicsRequestRef,
      setAuthorError,
      setBookmarkedPosts,
      setBookmarkedReactionAssets,
      setChannelPanelStateByTopic,
      setCommunityNodeConfig,
      setCommunityNodeInput,
      setCommunityNodeStatuses,
      setDirectMessageError,
      setDirectMessages,
      setDirectMessageStatusByPeer,
      setDirectMessageTimelineByPeer,
      setDiscoveryConfig,
      setDiscoverySeedInput,
      setError,
      setGamePanelStateByTopic,
      setGameRoomsByTopic,
      setJoinedChannelsByTopic,
      setKnownAuthorsByPubkey,
      setLivePanelStateByTopic,
      setLiveSessionsByTopic,
      setLocalPeerTicket,
      setLocalProfile,
      setOwnedReactionAssets,
      setProfileDraft,
      setProfileError,
      setProfilePanelState,
      setProfileTimeline,
      setPublicTimelinesByTopic,
      setReactionPanelState,
      setRecentReactions,
      setSelectedAuthor,
      setSelectedAuthorTimeline,
      setSelectedDirectMessagePeerPubkey,
      setSocialConnections,
      setSocialConnectionsPanelState,
      setSyncStatus,
      setThread,
      setTimelinesByTopic,
      storeApi,
      translate,
    ]
  );

  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (disposed) {
        return;
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [
    activeTopic,
    directMessagePaneOpen,
    loadTopics,
    selectedAuthorPubkey,
    selectedDirectMessagePeerPubkey,
    selectedThread,
    trackedTopics,
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
