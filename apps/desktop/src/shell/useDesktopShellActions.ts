import { type Dispatch, type FormEvent, type SetStateAction } from 'react';

import type {
  AttachmentView,
  DesktopApi,
  LocalDraftMediaItem,
  LocalPostDraft,
  PostView,
} from '@/lib/api';

import {
  activeTimelineStorageKey,
  PUBLIC_CHANNEL_REF,
  timelineStorageKeyForChannel,
  type DraftMediaItem,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  useDesktopShellStoreApi,
} from '@/shell/store';
import { publishedTopicIdForPost } from '@/shell/selectors';
import { createComposeInteractionsActions } from './actions/composeInteractions';
import { createDirectMessageActions } from './actions/directMessages';
import { createLiveGameActions } from './actions/liveGame';
import { createMessageReactionSocialActions } from './actions/messageReactionSocial';
import { createProfileTopicChannelActions } from './actions/profileTopicChannel';
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
  refreshVisibleTimelineAfterPublish: (topic: string, currentThread: string | null) => Promise<void>;
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
  const state = useDesktopShellStore();
  const nextActiveTopic = state.activeTopic;
  const nextSelectedChannelId = state.selectedChannelIdByTopic[nextActiveTopic] ?? null;
  const nextJoinedChannels = state.joinedChannelsByTopic[nextActiveTopic] ?? [];
  const activeComposeChannel = state.repostTarget
    ? PUBLIC_CHANNEL_REF
    : state.replyTarget?.channel_id
      ? {
          kind: 'private_channel' as const,
          channel_id: state.replyTarget.channel_id,
        }
      : state.composeChannelByTopic[nextActiveTopic] ?? PUBLIC_CHANNEL_REF;
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
  const setError = useDesktopShellFieldSetter('error');

  function cloneDraftMediaItems(items: DraftMediaItem[]): LocalDraftMediaItem[] {
    return items.map((item) => ({
      id: item.id,
      source_name: item.source_name,
      preview_url: item.preview_url,
      attachments: item.attachments.map((attachment) => ({ ...attachment })),
    }));
  }

  function attachmentViewsFromDraftMediaItems(
    localId: string,
    items: LocalDraftMediaItem[]
  ): AttachmentView[] {
    let attachmentIndex = 0;
    return items.flatMap((item) =>
      item.attachments.map((attachment) => ({
        hash: `${localId}-attachment-${attachmentIndex++}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }))
    );
  }

  function prependPost(posts: PostView[], post: PostView) {
    return [post, ...posts.filter((current) => current.object_id !== post.object_id)];
  }

  function patchLocalPosts(
    localId: string,
    updater: (post: PostView) => PostView,
    topicId: string = activeTopic
  ) {
    const patch = (posts: PostView[]) =>
      posts.map((post) => (post.local_id === localId ? updater(post) : post));

    setTimelinesByKey((current) => {
      let changed = false;
      const next = { ...current };
      for (const [key, posts] of Object.entries(current)) {
        if (!key.startsWith(`${topicId}::`) || !posts.some((post) => post.local_id === localId)) {
          continue;
        }
        next[key] = patch(posts);
        changed = true;
      }
      return changed ? next : current;
    });
    setPublicTimelinesByTopic((current) => ({
      ...current,
      [topicId]: patch(current[topicId] ?? []),
    }));
    setThread((current) => patch(current));
    setProfileTimeline((current) => patch(current));
    setSelectedAuthorTimeline((current) => patch(current));
  }

  function findKnownPost(objectId: string): PostView | null {
    const currentState = storeApi.getState();
    const lists = [
      currentState.thread,
      currentState.timelinesByKey[activeTimelineStorageKey(currentState, currentState.activeTopic)] ?? [],
      currentState.publicTimelinesByTopic[currentState.activeTopic] ?? [],
      currentState.profileTimeline,
      currentState.selectedAuthorTimeline,
    ];
    for (const posts of lists) {
      const match = posts.find((post) => post.object_id === objectId);
      if (match) {
        return match;
      }
    }
    return null;
  }

  function restoreLocalDraft(post: PostView) {
    const draft = post.local_draft;
    if (!draft) {
      return;
    }
    const draftMedia = cloneDraftMediaItems(post.local_draft_media_items ?? []);
    for (const item of draftMedia) {
      rememberDraftPreview(item as DraftMediaItem);
    }
    if (draft.topic !== activeTopic) {
      setActiveTopic(draft.topic);
    }
    setComposer(draft.content);
    setDraftMediaItems(draftMedia as DraftMediaItem[]);
    setAttachmentInputKey((value) => value + 1);
    setComposeChannelByTopic((current) => ({
      ...current,
      [draft.topic]:
        draft.kind === 'repost' ? PUBLIC_CHANNEL_REF : (draft.channel_ref ?? PUBLIC_CHANNEL_REF),
    }));
    if (draft.kind === 'repost' && draft.source_object_id) {
      setRepostTarget(findKnownPost(draft.source_object_id));
      setReplyTarget(null);
    } else if (draft.reply_to) {
      setReplyTarget(findKnownPost(draft.reply_to));
      setRepostTarget(null);
      setSelectedThread(post.root_id ?? draft.reply_to);
    } else {
      setReplyTarget(null);
      setRepostTarget(null);
      const channelRef = draft.channel_ref;
      if (channelRef && channelRef.kind === 'private_channel') {
        setSelectedChannelIdByTopic((current) => ({
          ...current,
          [draft.topic]: channelRef.channel_id,
        }));
      }
    }
    setComposerError(post.local_error ?? null);
    setComposeDialogOpen(true);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
    }));
    syncRoute('replace', {
      activeTopic: draft.topic,
      primarySection: 'timeline',
      selectedThread: draft.reply_to ? post.root_id ?? draft.reply_to : null,
    });
  }

  function createOptimisticPost(args: {
    createdAt: number;
    localId: string;
    draft: LocalPostDraft;
    draftMedia: LocalDraftMediaItem[];
    replyPost?: PostView | null;
    repostPost?: PostView | null;
  }): PostView {
    const { createdAt, localId, draft, draftMedia, replyPost = null, repostPost = null } = args;
    const isRepost = draft.kind === 'repost' && repostPost;
    const channelId =
      draft.kind === 'post' && draft.channel_ref?.kind === 'private_channel'
        ? draft.channel_ref.channel_id
        : null;
    const rootId = replyPost ? replyPost.root_id ?? replyPost.object_id : localId;
    return {
      object_id: localId,
      envelope_id: localId,
      author_pubkey: localAuthorPubkey,
      author_name: localProfile?.name ?? null,
      author_display_name: localProfile?.display_name ?? null,
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: false,
      object_kind: isRepost ? 'repost' : replyPost ? 'comment' : 'post',
      content: draft.content,
      content_status: 'Available',
      attachments: attachmentViewsFromDraftMediaItems(localId, draftMedia),
      created_at: createdAt,
      reply_to: replyPost?.object_id ?? null,
      reply_preview: replyPost
        ? {
            object_id: replyPost.object_id,
            topic: publishedTopicIdForPost(replyPost) ?? draft.topic,
            author: {
              pubkey: replyPost.author_pubkey,
              name: replyPost.author_name ?? null,
              display_name: replyPost.author_display_name ?? null,
              picture: replyPost.author_picture ?? null,
              picture_asset: replyPost.author_picture_asset ?? null,
            },
            content: replyPost.content,
            attachments: replyPost.attachments.map((attachment) => ({ ...attachment })),
            root_id: replyPost.root_id ?? null,
            reply_to: replyPost.reply_to ?? null,
          }
        : null,
      root_id: isRepost ? null : rootId,
      published_topic_id: draft.topic,
      origin_topic_id: draft.topic,
      repost_of: repostPost
        ? {
            source_object_id: repostPost.object_id,
            source_topic_id: publishedTopicIdForPost(repostPost) ?? draft.topic,
            source_author_pubkey: repostPost.author_pubkey,
            source_author_name: repostPost.author_name ?? null,
            source_author_display_name: repostPost.author_display_name ?? null,
            source_object_kind: repostPost.object_kind,
            content: repostPost.content,
            attachments: repostPost.attachments.map((attachment) => ({ ...attachment })),
            reply_to: repostPost.reply_to ?? null,
            root_id: repostPost.root_id ?? null,
          }
        : null,
      repost_commentary: repostPost ? (draft.content.trim() || null) : null,
      is_threadable: repostPost ? Boolean(draft.content.trim()) : true,
      channel_id: channelId,
      audience_label:
        replyPost?.audience_label ??
        (channelId ? activePrivateChannel?.label ?? 'Private channel' : 'Public'),
      reaction_summary: [],
      my_reactions: [],
      local_id: localId,
      local_state: 'pending',
      local_error: null,
      server_object_id: null,
      local_draft: {
        ...draft,
        attachments: draft.attachments?.map((attachment) => ({ ...attachment })) ?? [],
      },
      local_draft_media_items: draftMedia,
    };
  }

  function insertOptimisticPost(post: PostView) {
    const currentState = storeApi.getState();
    const selectedChannelId = currentState.selectedChannelIdByTopic[post.published_topic_id ?? activeTopic] ?? null;
    const topicId = post.published_topic_id ?? activeTopic;
    const timelineKey = timelineStorageKeyForChannel(topicId, post.channel_id ?? null);
    const belongsToActiveTimeline = post.channel_id
      ? selectedChannelId === post.channel_id
      : selectedChannelId === null;

    if (belongsToActiveTimeline) {
      setTimelinesByKey((current) => ({
        ...current,
        [timelineKey]: prependPost(current[timelineKey] ?? [], post),
      }));
    }
    if (!post.channel_id) {
      setPublicTimelinesByTopic((current) => ({
        ...current,
        [topicId]: prependPost(current[topicId] ?? [], post),
      }));
    }
    if (post.root_id && currentState.selectedThread === post.root_id) {
      setThread((current) => prependPost(current, post));
    }
    if (!post.channel_id && localProfile && currentState.shellChromeState.activePrimarySection === 'profile') {
      setProfileTimeline((current) => prependPost(current, post));
    }
    if (
      !post.channel_id &&
      currentState.selectedAuthorPubkey === localAuthorPubkey &&
      currentState.shellChromeState.activePrimarySection === 'timeline'
    ) {
      setSelectedAuthorTimeline((current) => prependPost(current, post));
    }
  }

  async function submitOptimisticPost(post: PostView) {
    const draft = post.local_draft;
    const draftMedia = cloneDraftMediaItems(post.local_draft_media_items ?? []);
    if (!draft || !post.local_id) {
      return;
    }
    patchLocalPosts(post.local_id, (current) => ({
      ...current,
      local_state: 'pending',
      local_error: null,
    }), draft.topic);
    try {
      const serverObjectId =
        draft.kind === 'repost' && draft.source_topic && draft.source_object_id
          ? await api.createRepost(
              draft.topic,
              draft.source_topic,
              draft.source_object_id,
              draft.content.trim() || null
            )
          : await api.createPost(
              draft.topic,
              draft.content,
              draft.reply_to ?? null,
              draft.attachments ?? [],
              draft.channel_ref ?? PUBLIC_CHANNEL_REF
            );
      for (const item of draftMedia) {
        releaseDraftPreview(item.id);
      }
      patchLocalPosts(
        post.local_id,
        (current) => ({
          ...current,
          local_state: 'syncing',
          local_error: null,
          server_object_id: serverObjectId,
        }),
        draft.topic
      );
      void refreshVisibleTimelineAfterPublish(
        draft.topic,
        draft.reply_to ? post.root_id ?? draft.reply_to : null
      );
    } catch (publishError) {
      const message =
        publishError instanceof Error
          ? publishError.message
          : translate('common:errors.failedToPublish');
      patchLocalPosts(
        post.local_id,
        (current) => ({
          ...current,
          local_state: 'failed',
          local_error: message,
        }),
        draft.topic
      );
      setComposerError(message);
    }
  }

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
    handleCreatePrivateChannel,
    handleShareChannelAccess,
    handleJoinChannelAccess,
    handleImportChannelAccessToken,
    handleSaveDiscoverySeeds,
    handleSaveCommunityNodes,
    handleClearCommunityNodes,
    handleAuthenticateCommunityNode,
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
    submitOptimisticPost,
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

  const { handleSendDirectMessage } = createDirectMessageActions({
    api,
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

  return {
    handleProfileFieldChange,
    handleProfileAvatarFile,
    handleClearProfileAvatar,
    resetProfileDraft,
    handleSaveProfile,
    handleAddTopic,
    handleSelectTopic,
    handleOpenOriginalTopic,
    handleRemoveTopic,
    handleSelectPrivateChannel,
    handleCreatePrivateChannel,
    handleShareChannelAccess,
    handleJoinChannelAccess,
    handleImportChannelAccessToken,
    handlePublish,
    handleAttachmentSelection,
    handleRemoveDraftAttachment,
    handleDirectMessageAttachmentSelection,
    handleRemoveDirectMessageDraftAttachment,
    handleSendDirectMessage,
    handleDeleteDirectMessageMessage,
    handleClearDirectMessage,
    handleOpenNotification,
    handleToggleReaction,
    handleCreateCustomReactionAsset,
    handleBookmarkCustomReaction,
    handleRemoveBookmarkedCustomReaction,
    handleToggleBookmarkedPost,
    beginReply,
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
