import {
  useMemo,
  type ChangeEvent,
  type Dispatch,
  type FormEvent,
  type SetStateAction,
} from 'react';

import type { DesktopApi, PostView } from '@/lib/api';

import {
  PUBLIC_CHANNEL_REF,
  timelineStorageKeyForChannel,
  type DraftMediaItem,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  useDesktopShellStoreApi,
} from '@/shell/store';
import { setRecordEntry } from '@/shell/stateUpdates';
import {
  messageFromError,
  privateTimelineScope,
  publishedTopicIdForPost,
} from '@/shell/presentation';
import { selectShellActionsSlice } from '@/shell/storeSelectors';
import {
  columnDraftKey,
  removeColumnDraft,
  setColumnDraft,
  type ColumnDraftTarget,
} from '@/shell/slices/columnDrafts';
import { columnIdentityId, openTransientColumn } from '@/shell/slices/workspace';
import { createComposeInteractionsActions } from './actions/composeInteractions';
import { createDirectMessageActions } from './actions/directMessages';
import { createLiveGameActions } from './actions/liveGame';
import { createMessageReactionSocialActions } from './actions/messageReactionSocial';
import { createMetaverseRoomActions } from './actions/metaverse';
import { createOptimisticPostActions } from './actions/optimisticPosts';
import { createProfileTopicChannelActions } from './actions/profileTopicChannel';
import { useShallow } from 'zustand/react/shallow';
import type {
  OpenAuthorDetail,
  OpenDirectMessagePane,
  OpenThread,
  SyncRoute,
  Translate,
} from './actions/shared';

type UseDesktopShellActionsArgs = {
  api: DesktopApi;
  translate: Translate;
  loadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>;
  refreshVisibleTimelineAfterPublish: (
    topic: string,
    currentThread: string | null,
    scopeChannelId?: string | null
  ) => Promise<void>;
  syncRoute: SyncRoute;
  openDirectMessagePane: OpenDirectMessagePane;
  openAuthorDetail: OpenAuthorDetail;
  openThread: OpenThread;
  setComposeDialogOpen: Dispatch<SetStateAction<boolean>>;
  setLiveCreateDialogOpen: Dispatch<SetStateAction<boolean>>;
  setGameCreateDialogOpen: Dispatch<SetStateAction<boolean>>;
  setProfileAvatarPreviewUrl: Dispatch<SetStateAction<string | null>>;
  setProfileAvatarInputKey: Dispatch<SetStateAction<number>>;
  releaseDraftPreview: (itemId: string) => void;
  releaseAllDraftPreviews: () => void;
  rememberDraftPreview: (item: DraftMediaItem) => void;
  releaseDirectMessageDraftPreview: (itemId: string) => void;
  releaseAllDirectMessageDraftPreviews: () => void;
  rememberDirectMessageDraftPreview: (item: DraftMediaItem) => void;
  buildImageDraftItem: (file: File) => Promise<DraftMediaItem>;
  buildVideoDraftItem: (file: File) => Promise<DraftMediaItem>;
};

export function useDesktopShellActions({
  api,
  translate,
  loadTopics,
  refreshVisibleTimelineAfterPublish,
  syncRoute,
  openDirectMessagePane,
  openAuthorDetail,
  openThread,
  setComposeDialogOpen,
  setLiveCreateDialogOpen,
  setGameCreateDialogOpen,
  setProfileAvatarPreviewUrl,
  setProfileAvatarInputKey,
  releaseDraftPreview,
  releaseAllDraftPreviews,
  rememberDraftPreview,
  releaseDirectMessageDraftPreview,
  releaseAllDirectMessageDraftPreviews,
  rememberDirectMessageDraftPreview,
  buildImageDraftItem,
  buildVideoDraftItem,
}: UseDesktopShellActionsArgs) {
  const storeApi = useDesktopShellStoreApi();
  const state = useDesktopShellStore(useShallow(selectShellActionsSlice));
  const nextActiveTopic = state.activeTopic;
  const nextSelectedChannelId = state.selectedChannelIdByTopic[nextActiveTopic] ?? null;
  const nextJoinedChannels = state.joinedChannelsByTopic[nextActiveTopic] ?? [];
  const activeComposeChannel = useMemo(
    () =>
      state.repostTarget
        ? PUBLIC_CHANNEL_REF
        : state.replyTarget?.channel_id
          ? {
              kind: 'private_channel' as const,
              channel_id: state.replyTarget.channel_id,
            }
          : state.composeChannelByTopic[nextActiveTopic] ?? PUBLIC_CHANNEL_REF,
    [nextActiveTopic, state.composeChannelByTopic, state.replyTarget, state.repostTarget]
  );
  const {
    trackedTopics,
    activeTopic,
    topicInput,
    composer,
    draftMediaItems,
    repostTarget,
    replyTarget,
    selectedThread,
    selectedChannelIdByTopic,
    channelLabelInput,
    channelAudienceInput,
    inviteTokenInput,
    gameDrafts,
    liveTitle,
    liveDescription,
    gameTitle,
    gameDescription,
    gameParticipantsInput,
    peerTicket,
    discoverySeedInput,
    communityNodeInput,
    localProfile,
    profileDraft,
    selectedAuthorPubkey,
    shellChromeState,
  } = state;
  const activePrivateChannel =
    nextJoinedChannels.find((channel) => channel.channel_id === nextSelectedChannelId) ?? null;
  const bookmarkedPostIds = new Set(state.bookmarkedPosts.map((item) => item.post.object_id));
  const activeGameRooms = state.gameRoomsByTopic[nextActiveTopic] ?? [];
  const localAuthorPubkey = state.syncStatus.local_author_pubkey;
  const metaverseActions = useMemo(
    () =>
      createMetaverseRoomActions({
        api,
        activeTopic,
        activeComposeChannel,
        onRefresh: () => loadTopics(trackedTopics, activeTopic, selectedThread),
      }),
    [api, activeComposeChannel, activeTopic, loadTopics, selectedThread, trackedTopics]
  );

  const setTrackedTopics = useDesktopShellFieldSetter('trackedTopics');
  const setActiveTopic = useDesktopShellFieldSetter('activeTopic');
  const setTopicInput = useDesktopShellFieldSetter('topicInput');
  const setComposer = useDesktopShellFieldSetter('composer');
  const setDraftMediaItems = useDesktopShellFieldSetter('draftMediaItems');
  const setAttachmentInputKey = useDesktopShellFieldSetter('attachmentInputKey');
  const setTimelinesByKey = useDesktopShellFieldSetter('timelinesByKey');
  const setPublicTimelinesByTopic = useDesktopShellFieldSetter('publicTimelinesByTopic');
  const setJoinedChannelsByTopic = useDesktopShellFieldSetter('joinedChannelsByTopic');
  const setSelectedChannelIdByTopic = useDesktopShellFieldSetter('selectedChannelIdByTopic');
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setSelectedThread = useDesktopShellFieldSetter('selectedThread');
  const setThread = useDesktopShellFieldSetter('thread');
  const setReplyTarget = useDesktopShellFieldSetter('replyTarget');
  const setRepostTarget = useDesktopShellFieldSetter('repostTarget');
  const setPeerTicket = useDesktopShellFieldSetter('peerTicket');
  const setDiscoveryConfig = useDesktopShellFieldSetter('discoveryConfig');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setDiscoveryEditorDirty = useDesktopShellFieldSetter('discoveryEditorDirty');
  const setDiscoveryError = useDesktopShellFieldSetter('discoveryError');
  const setCommunityNodeConfig = useDesktopShellFieldSetter('communityNodeConfig');
  const setCommunityNodeStatuses = useDesktopShellFieldSetter('communityNodeStatuses');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setCommunityNodeEditorDirty = useDesktopShellFieldSetter('communityNodeEditorDirty');
  const setCommunityNodeError = useDesktopShellFieldSetter('communityNodeError');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const setOwnedReactionAssets = useDesktopShellFieldSetter('ownedReactionAssets');
  const setBookmarkedReactionAssets = useDesktopShellFieldSetter('bookmarkedReactionAssets');
  const setBookmarkedPosts = useDesktopShellFieldSetter('bookmarkedPosts');
  const setRecentReactions = useDesktopShellFieldSetter('recentReactions');
  const setProfileDraft = useDesktopShellFieldSetter('profileDraft');
  const setProfileDirty = useDesktopShellFieldSetter('profileDirty');
  const setProfileError = useDesktopShellFieldSetter('profileError');
  const setProfilePanelState = useDesktopShellFieldSetter('profilePanelState');
  const setProfileSaving = useDesktopShellFieldSetter('profileSaving');
  const setLocalProfile = useDesktopShellFieldSetter('localProfile');
  const setProfileTimeline = useDesktopShellFieldSetter('profileTimeline');
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
  const setDirectMessageComposer = useDesktopShellFieldSetter('directMessageComposer');
  const setDirectMessageDraftMediaItems = useDesktopShellFieldSetter('directMessageDraftMediaItems');
  const setDirectMessageAttachmentInputKey = useDesktopShellFieldSetter(
    'directMessageAttachmentInputKey'
  );
  const setDirectMessageError = useDesktopShellFieldSetter('directMessageError');
  const setDirectMessageSending = useDesktopShellFieldSetter('directMessageSending');
  const setComposerError = useDesktopShellFieldSetter('composerError');
  const setColumnDraftsByKey = useDesktopShellFieldSetter('columnDraftsByKey');
  const setLiveTitle = useDesktopShellFieldSetter('liveTitle');
  const setLiveDescription = useDesktopShellFieldSetter('liveDescription');
  const setLiveError = useDesktopShellFieldSetter('liveError');
  const setLivePendingBySessionId = useDesktopShellFieldSetter('livePendingBySessionId');
  const setLiveCreatePending = useDesktopShellFieldSetter('liveCreatePending');
  const setChannelLabelInput = useDesktopShellFieldSetter('channelLabelInput');
  const setChannelAudienceInput = useDesktopShellFieldSetter('channelAudienceInput');
  const setInviteTokenInput = useDesktopShellFieldSetter('inviteTokenInput');
  const setInviteOutput = useDesktopShellFieldSetter('inviteOutput');
  const setInviteOutputLabel = useDesktopShellFieldSetter('inviteOutputLabel');
  const setChannelError = useDesktopShellFieldSetter('channelError');
  const setChannelPanelStateByTopic = useDesktopShellFieldSetter('channelPanelStateByTopic');
  const setChannelActionPending = useDesktopShellFieldSetter('channelActionPending');
  const setGameTitle = useDesktopShellFieldSetter('gameTitle');
  const setGameDescription = useDesktopShellFieldSetter('gameDescription');
  const setGameParticipantsInput = useDesktopShellFieldSetter('gameParticipantsInput');
  const setGameError = useDesktopShellFieldSetter('gameError');
  const setGameDrafts = useDesktopShellFieldSetter('gameDrafts');
  const setGameSavingByRoomId = useDesktopShellFieldSetter('gameSavingByRoomId');
  const setGameCreatePending = useDesktopShellFieldSetter('gameCreatePending');
  const setReactionPanelState = useDesktopShellFieldSetter('reactionPanelState');
  const setReactionCreatePending = useDesktopShellFieldSetter('reactionCreatePending');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');
  const setWorkspaceState = useDesktopShellFieldSetter('workspaceState');
  const setError = useDesktopShellFieldSetter('error');

  const {
    cloneDraftMediaItems,
    createOptimisticPost,
    insertOptimisticPost,
    restoreLocalDraft,
    submitOptimisticPost,
  } = createOptimisticPostActions({
    api,
    activeTopic,
    localAuthorPubkey,
    localProfile,
    refreshVisibleTimelineAfterPublish,
    releaseDraftPreview,
    rememberDraftPreview,
    storeApi,
    syncRoute,
    translate,
    setActiveTopic,
    setAttachmentInputKey,
    setComposeChannelByTopic,
    setComposeDialogOpen,
    setComposer,
    setComposerError,
    setDraftMediaItems,
    setProfileTimeline,
    setPublicTimelinesByTopic,
    setReplyTarget,
    setRepostTarget,
    setSelectedAuthorTimeline,
    setSelectedChannelIdByTopic,
    setSelectedThread,
    setShellChromeState,
    setThread,
    setTimelinesByKey,
  });

  function clearThreadContext() {
    setSelectedThread(null);
    setThread([]);
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setSelectedAuthorTimeline([]);
    setAuthorError(null);
  }

  function clearAuxiliaryPanels() {
    clearThreadContext();
    setDirectMessagePaneOpen(false);
    setSelectedDirectMessagePeerPubkey(null);
    setDirectMessageError(null);
  }

  const {
    handleProfileFieldChange,
    handleProfileAvatarFile,
    handleClearProfileAvatar,
    resetProfileDraft,
    handleSelectPrivateChannel,
    handleSaveProfile,
    handleAddTopic,
    handleSelectTopic,
    handleOpenOriginalTopic,
    handleRemoveTopic,
    handleToggleTopicGossip,
    handleToggleChannelGossip,
    handleCreatePrivateChannel,
    handleLeavePrivateChannel,
    handleShareChannelAccess,
    handleJoinChannelAccess,
    handleImportChannelAccessToken,
    handleSaveDiscoverySeeds,
    handleSaveCommunityNodes,
    handleClearCommunityNodes,
    handleAuthenticateCommunityNode,
    handleSetCommunityNodeInviteCode,
    handleClearCommunityNodeToken,
    handleRefreshCommunityNode,
    handleFetchCommunityNodeConsents,
    handleAcceptCommunityNodeConsents,
  } = createProfileTopicChannelActions({
    api,
    translate,
    loadTopics,
    syncRoute,
    activePrivateChannel,
    activeTopic,
    channelAudienceInput,
    channelLabelInput,
    communityNodeInput,
    discoverySeedInput,
    inviteTokenInput,
    localProfile,
    profileDraft,
    selectedChannelIdByTopic,
    selectedThread,
    topicInput,
    trackedTopics,
    clearThreadContext,
    setProfileAvatarPreviewUrl,
    setProfileAvatarInputKey,
    setTrackedTopics,
    setActiveTopic,
    setTopicInput,
    setTimelineScopeByTopic,
    setComposeChannelByTopic,
    setSelectedChannelIdByTopic,
    setShellChromeState,
    setProfileDraft,
    setProfileDirty,
    setProfileError,
    setProfilePanelState,
    setProfileSaving,
    setLocalProfile,
    setChannelLabelInput,
    setChannelAudienceInput,
    setInviteTokenInput,
    setInviteOutput,
    setInviteOutputLabel,
    setChannelError,
    setChannelPanelStateByTopic,
    setChannelActionPending,
    setJoinedChannelsByTopic,
    setCommunityNodeConfig,
    setCommunityNodeStatuses,
    setCommunityNodeInput,
    setCommunityNodeEditorDirty,
    setCommunityNodeError,
    setDiscoveryConfig,
    setDiscoverySeedInput,
    setDiscoveryEditorDirty,
    setDiscoveryError,
  });

  const {
    handleDeleteDirectMessageMessage,
    handleClearDirectMessage,
    handleOpenNotification,
    handleToggleReaction,
    handleCreateCustomReactionAsset,
    handleBookmarkCustomReaction,
    handleRemoveBookmarkedCustomReaction,
    handleToggleBookmarkedPost,
    handleRelationshipAction,
    handleMuteAction,
  } = createMessageReactionSocialActions({
    api,
    translate,
    loadTopics,
    syncRoute,
    openDirectMessagePane,
    openAuthorDetail,
    openThread,
    activeTopic,
    bookmarkedPostIds,
    selectedAuthorPubkey,
    selectedThread,
    trackedTopics,
    clearAuxiliaryPanels,
    setTrackedTopics,
    setActiveTopic,
    setSelectedChannelIdByTopic,
    setTimelineScopeByTopic,
    setComposeChannelByTopic,
    setTimelinesByKey,
    setPublicTimelinesByTopic,
    setThread,
    setProfileTimeline,
    setSelectedAuthorTimeline,
    setKnownAuthorsByPubkey,
    setOwnedReactionAssets,
    setBookmarkedReactionAssets,
    setBookmarkedPosts,
    setRecentReactions,
    setSelectedAuthor,
    setAuthorError,
    setDirectMessageError,
    setReactionPanelState,
    setReactionCreatePending,
    setShellChromeState,
    setError,
  });

  const {
    handleImportPeer,
    handleCreateLiveSession,
    handleJoinLiveSession,
    handleLeaveLiveSession,
    handleEndLiveSession,
    handleCreateGameRoom,
    updateGameDraft,
    handleUpdateGameRoom,
  } = createLiveGameActions({
    api,
    translate,
    loadTopics,
    syncRoute,
    activeComposeChannel,
    activeGameRooms,
    activeTopic,
    gameDescription,
    gameDrafts,
    gameParticipantsInput,
    gameTitle,
    liveDescription,
    liveTitle,
    peerTicket,
    selectedThread,
    trackedTopics,
    setPeerTicket,
    setLiveTitle,
    setLiveDescription,
    setLiveError,
    setLivePendingBySessionId,
    setLiveCreatePending,
    setShellChromeState,
    setGameTitle,
    setGameDescription,
    setGameParticipantsInput,
    setGameError,
    setGameDrafts,
    setGameSavingByRoomId,
    setGameCreatePending,
    setError,
    setLiveCreateDialogOpen,
    setGameCreateDialogOpen,
  });

  const {
    handleAttachmentSelection,
    handleRemoveDraftAttachment,
    handleDirectMessageAttachmentSelection,
    handleRemoveDirectMessageDraftAttachment,
    beginReply,
    clearReply,
    clearRepost,
    openNewPostDialog,
    openFloatingActionDialog,
    handleSimpleRepost,
    handleRestoreLocalPost,
    handleRetryLocalPost,
    beginQuoteRepost,
  } = createComposeInteractionsActions({
    activeTopic,
    buildImageDraftItem,
    buildVideoDraftItem,
    createOptimisticPost,
    insertOptimisticPost,
    openThread,
    releaseAllDirectMessageDraftPreviews,
    releaseAllDraftPreviews,
    releaseDirectMessageDraftPreview,
    releaseDraftPreview,
    rememberDirectMessageDraftPreview,
    rememberDraftPreview,
    restoreLocalDraft,
    shellChromeState,
    submitOptimisticPost: async (post) => {
      await submitOptimisticPost(post);
    },
    syncRoute,
    translate,
    setAttachmentInputKey,
    setAuthorError,
    setComposer,
    setComposerError,
    setDirectMessageAttachmentInputKey,
    setDirectMessageDraftMediaItems,
    setDirectMessageError,
    setDraftMediaItems,
    setReplyTarget,
    setRepostTarget,
    setSelectedAuthor,
    setSelectedAuthorPubkey,
    setSelectedThread,
    setShellChromeState,
    setThread,
    setComposeDialogOpen,
    setGameCreateDialogOpen,
    setLiveCreateDialogOpen,
  });

  const { handleSendDirectMessage, sendDirectMessageDraft } = createDirectMessageActions({
    api,
    translate,
    getState: storeApi.getState,
    openDirectMessagePane,
    releaseAllDirectMessageDraftPreviews,
    setDirectMessageTimelineByPeer,
    setDirectMessages,
    setDirectMessageComposer,
    setDirectMessageDraftMediaItems,
    setDirectMessageAttachmentInputKey,
    setDirectMessageError,
    setDirectMessageSending,
  });

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmedComposer = composer.trim();
    const draftMediaSnapshot = cloneDraftMediaItems(draftMediaItems);
    const attachments = draftMediaSnapshot.flatMap((item) => item.attachments);
    if (repostTarget) {
      const sourceTopic = publishedTopicIdForPost(repostTarget);
      if (!sourceTopic) {
        setComposerError(translate('common:errors.failedToPublish'));
        return;
      }
      if (!trimmedComposer) {
        setComposerError(translate('common:errors.quoteRepostRequiresCommentary'));
        return;
      }
      const createdAt = Math.floor(Date.now() / 1000);
      const localId = `local-post:${Date.now()}:${Math.random().toString(16).slice(2)}`;
      const optimisticPost = createOptimisticPost({
        createdAt,
        localId,
        draft: {
          kind: 'repost',
          topic: activeTopic,
          content: trimmedComposer,
          source_topic: sourceTopic,
          source_object_id: repostTarget.object_id,
          channel_ref: PUBLIC_CHANNEL_REF,
        },
        draftMedia: [],
        repostPost: repostTarget,
      });
      insertOptimisticPost(optimisticPost);
      setComposer('');
      setDraftMediaItems([]);
      setAttachmentInputKey((value) => value + 1);
      setComposerError(null);
      setReplyTarget(null);
      setRepostTarget(null);
      setComposeDialogOpen(false);
      setSelectedThread(null);
      setThread([]);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
      }));
      syncRoute('replace', {
        primarySection: 'timeline',
        selectedThread: null,
      });
      void submitOptimisticPost(optimisticPost);
      return;
    }

    if (!trimmedComposer && attachments.length === 0) {
      return;
    }

    const createdAt = Math.floor(Date.now() / 1000);
    const localId = `local-post:${Date.now()}:${Math.random().toString(16).slice(2)}`;
    const optimisticPost = createOptimisticPost({
      createdAt,
      localId,
      draft: {
        kind: 'post',
        topic: activeTopic,
        content: trimmedComposer,
        reply_to: replyTarget?.object_id ?? null,
        channel_ref: activeComposeChannel,
        attachments,
      },
      draftMedia: draftMediaSnapshot,
      replyPost: replyTarget,
    });
    insertOptimisticPost(optimisticPost);
    setComposer('');
    setDraftMediaItems([]);
    setAttachmentInputKey((value) => value + 1);
    setComposerError(null);
    setComposeDialogOpen(false);
    setReplyTarget(null);
    setRepostTarget(null);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
    }));
    syncRoute('replace', {
      primarySection: 'timeline',
    });
    void submitOptimisticPost(optimisticPost);
  }

  async function handleColumnDraftAttachmentSelection(
    target: ColumnDraftTarget,
    event: ChangeEvent<HTMLInputElement>
  ) {
    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) return;
    const nextItems: DraftMediaItem[] = [];
    const failures: string[] = [];
    for (const file of files) {
      try {
        if (file.type.startsWith('image/')) {
          nextItems.push(await buildImageDraftItem(file));
        } else if (file.type.startsWith('video/')) {
          nextItems.push(await buildVideoDraftItem(file));
        } else {
          failures.push(translate('common:errors.unsupportedAttachmentType', { name: file.name }));
        }
      } catch (attachmentError) {
        failures.push(
          messageFromError(
            attachmentError,
            translate('common:errors.failedToGenerateVideoPoster')
          )
        );
      }
    }
    nextItems.forEach(rememberDraftPreview);
    setColumnDraftsByKey((current) =>
      setColumnDraft(current, target, (draft) => ({
        ...draft,
        mediaItems: [...draft.mediaItems, ...nextItems],
        error: failures[0] ?? null,
        attachmentInputKey: draft.attachmentInputKey + 1,
      }))
    );
  }

  function handleRemoveColumnDraftAttachment(target: ColumnDraftTarget, itemId: string) {
    releaseDraftPreview(itemId);
    setColumnDraftsByKey((current) =>
      setColumnDraft(current, target, (draft) => ({
        ...draft,
        mediaItems: draft.mediaItems.filter((item) => item.id !== itemId),
      }))
    );
  }

  async function handleSubmitColumnDraft(
    target: ColumnDraftTarget,
    event: FormEvent<HTMLFormElement>
  ) {
    event.preventDefault();
    const draft = storeApi.getState().columnDraftsByKey[columnDraftKey(target)];
    if (!draft || draft.pending) return;
    const trimmedContent = draft.content.trim();
    const draftMediaSnapshot = cloneDraftMediaItems(draft.mediaItems);
    const attachments = draftMediaSnapshot.flatMap((item) => item.attachments);
    if (!trimmedContent && attachments.length === 0) return;
    setColumnDraftsByKey((current) =>
      setColumnDraft(current, target, (currentDraft) => ({
        ...currentDraft,
        pending: true,
        error: null,
      }))
    );

    if (target.action === 'message') {
      if (!target.peerPubkey) return;
      const sent = await sendDirectMessageDraft(
        target.peerPubkey,
        trimmedContent,
        draft.mediaItems,
        () => draft.mediaItems.forEach((item) => releaseDraftPreview(item.id))
      );
      if (sent) {
        setColumnDraftsByKey((current) => removeColumnDraft(current, target));
      } else {
        const message =
          storeApi.getState().directMessageError ??
          translate('common:errors.failedToSendDirectMessage');
        setColumnDraftsByKey((current) =>
          setColumnDraft(current, target, (currentDraft) => ({
            ...currentDraft,
            pending: false,
            error: message,
          }))
        );
      }
      return;
    }

    if (!target.scope) return;
    const replyObjectId =
      target.action === 'reply'
        ? draft.replyTarget?.object_id ?? target.threadId ?? null
        : null;
    const currentState = storeApi.getState();
    const targetTimeline =
      currentState.timelinesByKey[
        timelineStorageKeyForChannel(target.scope.topicId, target.scope.channelId)
      ] ?? [];
    const replyPost = replyObjectId
      ? [...currentState.thread, ...targetTimeline].find(
          (post) => post.object_id === replyObjectId
        ) ?? draft.replyTarget
      : null;
    const createdAt = Math.floor(Date.now() / 1000);
    const localId = `local-post:${Date.now()}:${Math.random().toString(16).slice(2)}`;
    const optimisticPost = createOptimisticPost({
      createdAt,
      localId,
      draft: {
        kind: 'post',
        topic: target.scope.topicId,
        content: trimmedContent,
        reply_to: replyObjectId,
        channel_ref: target.scope.channelId
          ? { kind: 'private_channel', channel_id: target.scope.channelId }
          : PUBLIC_CHANNEL_REF,
        attachments,
      },
      draftMedia: draftMediaSnapshot,
      replyPost,
    });
    insertOptimisticPost(optimisticPost);
    const published = await submitOptimisticPost(optimisticPost);
    if (published) {
      setColumnDraftsByKey((current) => removeColumnDraft(current, target));
    } else {
      const failedPost = storeApi
        .getState()
        .timelinesByKey[
          timelineStorageKeyForChannel(target.scope.topicId, target.scope.channelId)
        ]?.find((post) => post.local_id === localId);
      setColumnDraftsByKey((current) =>
        setColumnDraft(current, target, (currentDraft) => ({
          ...currentDraft,
          pending: false,
          error: failedPost?.local_error ?? translate('common:errors.failedToPublish'),
        }))
      );
    }
  }

  function beginColumnReply(post: PostView) {
    const topicId = publishedTopicIdForPost(post) ?? activeTopic;
    const scope = { topicId, channelId: post.channel_id ?? null };
    const threadId = post.root_id ?? post.object_id;
    const timelineColumnId = columnIdentityId('timeline', scope);
    const threadColumnId = columnIdentityId('thread', scope, threadId);
    const target: ColumnDraftTarget = {
      columnId: threadColumnId,
      action: 'reply',
      scope,
      threadId,
    };
    setColumnDraftsByKey((current) =>
      setColumnDraft(current, target, (draft) => ({
        ...draft,
        expanded: true,
        replyTarget: post,
        error: null,
      }))
    );
    setWorkspaceState((current) =>
      openTransientColumn(current, {
        id: threadColumnId,
        kind: 'thread',
        scope,
        entityId: threadId,
        parentColumnId: timelineColumnId,
        pinned: false,
        preferredDesktopSpan: 1,
      })
    );
    setActiveTopic(topicId);
    setSelectedChannelIdByTopic(setRecordEntry(topicId, scope.channelId));
    setTimelineScopeByTopic(setRecordEntry(topicId, privateTimelineScope(scope.channelId)));
    void openThread(threadId, { topic: topicId });
  }

  return {
    metaverseActions,
    handleProfileFieldChange,
    handleProfileAvatarFile,
    handleClearProfileAvatar,
    resetProfileDraft,
    handleSaveProfile,
    handleAddTopic,
    handleSelectTopic,
    handleOpenOriginalTopic,
    handleRemoveTopic,
    handleToggleTopicGossip,
    handleToggleChannelGossip,
    handleSelectPrivateChannel,
    handleCreatePrivateChannel,
    handleLeavePrivateChannel,
    handleShareChannelAccess,
    handleJoinChannelAccess,
    handleImportChannelAccessToken,
    handlePublish,
    handleAttachmentSelection,
    handleRemoveDraftAttachment,
    handleDirectMessageAttachmentSelection,
    handleRemoveDirectMessageDraftAttachment,
    handleSendDirectMessage,
    handleColumnDraftAttachmentSelection,
    handleRemoveColumnDraftAttachment,
    handleSubmitColumnDraft,
    handleDeleteDirectMessageMessage,
    handleClearDirectMessage,
    handleOpenNotification,
    handleToggleReaction,
    handleCreateCustomReactionAsset,
    handleBookmarkCustomReaction,
    handleRemoveBookmarkedCustomReaction,
    handleToggleBookmarkedPost,
    beginReply,
    beginColumnReply,
    clearReply,
    clearRepost,
    openNewPostDialog,
    openFloatingActionDialog,
    handleSimpleRepost,
    handleRetryLocalPost,
    handleRestoreLocalPost,
    beginQuoteRepost,
    handleRelationshipAction,
    handleMuteAction,
    handleSaveDiscoverySeeds,
    handleSaveCommunityNodes,
    handleClearCommunityNodes,
    handleAuthenticateCommunityNode,
    handleSetCommunityNodeInviteCode,
    handleClearCommunityNodeToken,
    handleRefreshCommunityNode,
    handleFetchCommunityNodeConsents,
    handleAcceptCommunityNodeConsents,
    handleImportPeer,
    handleCreateLiveSession,
    handleJoinLiveSession,
    handleLeaveLiveSession,
    handleEndLiveSession,
    handleCreateGameRoom,
    updateGameDraft,
    handleUpdateGameRoom,
  };
}
