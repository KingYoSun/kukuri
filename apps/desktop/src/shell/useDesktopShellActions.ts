import {
  type ChangeEvent,
  type Dispatch,
  type FormEvent,
  type SetStateAction,
} from 'react';

import type {
  AttachmentView,
  CustomReactionCropRect,
  DesktopApi,
  DirectMessageConversationView,
  DirectMessageMessageView,
  GameScoreView,
  JoinedPrivateChannelView,
  PostView,
  ProfileInput,
  ReactionKeyInput,
} from '@/lib/api';
import { fileToCreateAttachment } from '@/lib/attachments';

import {
  DEFAULT_COMMUNITY_NODE_CONFIG,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  type DraftMediaItem,
  type GameEditorDraft,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  canCreateRepostFromPost,
  communityNodesToEditorValue,
  createGameEditorDraft,
  joinedChannelFromAccessTokenPreview,
  mergeKnownAuthors,
  messageFromError,
  patchReactionStateIntoPosts,
  privateComposeTarget,
  privateTimelineScope,
  profileInputFromProfile,
  publishedTopicIdForPost,
  seedPeersToEditorValue,
  syncCommunityNodeConfigWithStatus,
  upsertCommunityNodeStatus,
  upsertJoinedChannel,
} from '@/shell/selectors';
import type { OpenThreadOptions } from '@/shell/routes';

type UseDesktopShellActionsArgs = {
  api: DesktopApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
  loadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>;
  syncRoute: (mode?: 'push' | 'replace', overrides?: Record<string, unknown>) => void;
  openDirectMessagePane: (
    peerPubkey: string,
    options?: {
      historyMode?: 'push' | 'replace';
      normalizeOnError?: boolean;
      preserveAuthorPane?: boolean;
      preservedAuthorPubkey?: string | null;
    }
  ) => Promise<void>;
  openThread: (threadId: string, options?: OpenThreadOptions) => Promise<void>;
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
  syncRoute,
  openDirectMessagePane,
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
    directMessageComposer,
    directMessageDraftMediaItems,
    selectedDirectMessagePeerPubkey,
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
    selectedAuthor,
    shellChromeState,
  } = state;
  const activePrivateChannel =
    nextJoinedChannels.find((channel) => channel.channel_id === nextSelectedChannelId) ?? null;
  const bookmarkedPostIds = new Set(state.bookmarkedPosts.map((item) => item.post.object_id));
  const activeGameRooms = state.gameRoomsByTopic[nextActiveTopic] ?? [];

  const setTrackedTopics = useDesktopShellFieldSetter('trackedTopics');
  const setActiveTopic = useDesktopShellFieldSetter('activeTopic');
  const setTopicInput = useDesktopShellFieldSetter('topicInput');
  const setComposer = useDesktopShellFieldSetter('composer');
  const setDraftMediaItems = useDesktopShellFieldSetter('draftMediaItems');
  const setAttachmentInputKey = useDesktopShellFieldSetter('attachmentInputKey');
  const setTimelinesByTopic = useDesktopShellFieldSetter('timelinesByTopic');
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

  function directMessagePreviewFromAttachments(attachments: AttachmentView[]) {
    if (attachments.some((attachment) => attachment.role === 'video_manifest')) {
      return '[Video]';
    }
    if (attachments.length > 0) {
      return '[Image]';
    }
    return null;
  }

  function handleProfileFieldChange(field: 'displayName' | 'name' | 'about', value: string) {
    const nextField: keyof ProfileInput = field === 'displayName' ? 'display_name' : field;
    setProfileDraft((current) => ({
      ...current,
      [nextField]: value,
    }));
    setProfileDirty(true);
  }

  async function handleProfileAvatarSelection(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }
    const pictureUpload = await fileToCreateAttachment(file, 'profile_avatar');
    const nextPreviewUrl = URL.createObjectURL(file);
    setProfileAvatarPreviewUrl((current) => {
      if (current) {
        URL.revokeObjectURL(current);
      }
      return nextPreviewUrl;
    });
    setProfileAvatarInputKey((value) => value + 1);
    setProfileDraft((current) => ({
      ...current,
      picture: null,
      picture_upload: pictureUpload,
      clear_picture: false,
    }));
    setProfileDirty(true);
    setProfileError(null);
  }

  function handleClearProfileAvatar() {
    setProfileAvatarPreviewUrl((current) => {
      if (current) {
        URL.revokeObjectURL(current);
      }
      return null;
    });
    setProfileAvatarInputKey((value) => value + 1);
    setProfileDraft((current) => ({
      ...current,
      picture: null,
      picture_upload: null,
      clear_picture: true,
    }));
    setProfileDirty(true);
    setProfileError(null);
  }

  function resetProfileDraft() {
    if (!localProfile) {
      return;
    }
    setProfileAvatarPreviewUrl((current) => {
      if (current) {
        URL.revokeObjectURL(current);
      }
      return null;
    });
    setProfileAvatarInputKey((value) => value + 1);
    setProfileDraft(profileInputFromProfile(localProfile));
    setProfileDirty(false);
    setProfileError(null);
    setProfilePanelState({
      status: 'ready',
      error: null,
    });
  }

  function handleSelectPrivateChannel(topicId: string, channelId: string) {
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [topicId]: channelId,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'channel',
        channel_id: channelId,
      },
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'private_channel',
        channel_id: channelId,
      },
    }));
    setActiveTopic(topicId);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    syncRoute('replace', {
      activeTopic: topicId,
      primarySection: 'timeline',
      timelineScope: {
        kind: 'channel',
        channel_id: channelId,
      },
      composeTarget: {
        kind: 'private_channel',
        channel_id: channelId,
      },
    });
  }

  async function handleSaveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setProfileSaving(true);
    try {
      const profile = await api.setMyProfile(profileDraft);
      setProfileAvatarPreviewUrl((current) => {
        if (current) {
          URL.revokeObjectURL(current);
        }
        return null;
      });
      setProfileAvatarInputKey((value) => value + 1);
      setLocalProfile(profile);
      setProfileDraft(profileInputFromProfile(profile));
      setProfileDirty(false);
      setProfileError(null);
      setProfilePanelState({
        status: 'ready',
        error: null,
      });
      setShellChromeState((current) => ({
        ...current,
        profileMode: 'overview',
      }));
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace', {
        primarySection: 'profile',
        profileMode: 'overview',
      });
    } catch (saveError) {
      const nextProfileError = messageFromError(
        saveError,
        translate('common:errors.failedToSaveProfile')
      );
      setProfileError(nextProfileError);
      setProfilePanelState({
        status: 'error',
        error: nextProfileError,
      });
    } finally {
      setProfileSaving(false);
    }
  }

  async function handleAddTopic() {
    const nextTopic = topicInput.trim();
    if (!nextTopic) {
      return;
    }
    const nextTopics = trackedTopics.includes(nextTopic)
      ? trackedTopics
      : [...trackedTopics, nextTopic];
    setTrackedTopics(nextTopics);
    setActiveTopic(nextTopic);
    setTopicInput('');
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: nextTopic,
      primarySection: 'timeline',
    });
    await loadTopics(nextTopics, nextTopic, null);
  }

  async function handleSelectTopic(topic: string) {
    setActiveTopic(topic);
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [topic]: null,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [topic]: PUBLIC_TIMELINE_SCOPE,
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [topic]: PUBLIC_CHANNEL_REF,
    }));
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topic,
      primarySection: 'timeline',
      timelineScope: PUBLIC_TIMELINE_SCOPE,
      composeTarget: PUBLIC_CHANNEL_REF,
    });
    await loadTopics(trackedTopics, topic, null);
  }

  async function handleOpenOriginalTopic(topicId: string) {
    const nextTopics = trackedTopics.includes(topicId) ? trackedTopics : [...trackedTopics, topicId];
    setTrackedTopics(nextTopics);
    setActiveTopic(topicId);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topicId,
      primarySection: 'timeline',
      timelineScope: privateTimelineScope(selectedChannelIdByTopic[topicId] ?? null),
      composeTarget: privateComposeTarget(selectedChannelIdByTopic[topicId] ?? null),
      selectedAuthorPubkey: null,
      selectedThread: null,
    });
    await loadTopics(nextTopics, topicId, null);
  }

  async function handleRemoveTopic(topic: string) {
    if (trackedTopics.length === 1) {
      return;
    }
    const nextTopics = trackedTopics.filter((value) => value !== topic);
    const nextActiveTopic = activeTopic === topic ? nextTopics[0] : activeTopic;
    await api.unsubscribeTopic(topic);
    setTrackedTopics(nextTopics);
    setActiveTopic(nextActiveTopic);
    setShellChromeState((current) => ({
      ...current,
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: nextActiveTopic,
    });
    await loadTopics(nextTopics, nextActiveTopic, null);
  }

  async function handleCreatePrivateChannel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!channelLabelInput.trim()) {
      setChannelError(translate('channels:errors.channelLabelRequired'));
      return;
    }
    setChannelActionPending('create');
    try {
      const channel = await api.createPrivateChannel(
        activeTopic,
        channelLabelInput.trim(),
        channelAudienceInput
      );
      setJoinedChannelsByTopic((current) => ({
        ...current,
        [activeTopic]: upsertJoinedChannel(current[activeTopic] ?? [], channel),
      }));
      setChannelPanelStateByTopic((current) => ({
        ...current,
        [activeTopic]: {
          status: 'ready',
          error: null,
        },
      }));
      setChannelLabelInput('');
      setChannelAudienceInput('invite_only');
      setChannelError(null);
      setTimelineScopeByTopic((current) => ({
        ...current,
        [activeTopic]: {
          kind: 'channel',
          channel_id: channel.channel_id,
        },
      }));
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [activeTopic]: channel.channel_id,
      }));
      setComposeChannelByTopic((current) => ({
        ...current,
        [activeTopic]: {
          kind: 'private_channel',
          channel_id: channel.channel_id,
        },
      }));
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
        navOpen: false,
      }));
      syncRoute('replace', {
        activeTopic,
        composeTarget: {
          kind: 'private_channel',
          channel_id: channel.channel_id,
        },
        primarySection: 'timeline',
        timelineScope: {
          kind: 'channel',
          channel_id: channel.channel_id,
        },
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (channelCreateError) {
      setChannelError(
        messageFromError(channelCreateError, translate('channels:errors.failedCreateChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handleShareChannelAccess() {
    if (!activePrivateChannel) {
      setChannelError(translate('channels:errors.selectChannelForShare'));
      return;
    }
    setChannelActionPending('share');
    try {
      const access = await api.exportChannelAccessToken(activeTopic, activePrivateChannel.channel_id, null);
      setInviteOutput(access.token);
      setInviteOutputLabel(access.kind);
      setChannelError(null);
    } catch (shareError) {
      setChannelError(
        messageFromError(shareError, translate('channels:errors.failedShareChannel'))
      );
    } finally {
      setChannelActionPending(null);
    }
  }

  async function activateImportedPrivateChannel(
    topicId: string,
    channelId: string,
    placeholderChannel?: JoinedPrivateChannelView
  ) {
    const nextTopics = trackedTopics.includes(topicId) ? trackedTopics : [...trackedTopics, topicId];
    setTrackedTopics(nextTopics);
    setActiveTopic(topicId);
    if (placeholderChannel) {
      setJoinedChannelsByTopic((current) => ({
        ...current,
        [topicId]: upsertJoinedChannel(current[topicId] ?? [], placeholderChannel),
      }));
      setChannelPanelStateByTopic((current) => ({
        ...current,
        [topicId]: {
          status: 'ready',
          error: null,
        },
      }));
    }
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [topicId]: channelId,
    }));
    setTimelineScopeByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'channel',
        channel_id: channelId,
      },
    }));
    setComposeChannelByTopic((current) => ({
      ...current,
      [topicId]: {
        kind: 'private_channel',
        channel_id: channelId,
      },
    }));
    setInviteTokenInput('');
    setInviteOutput(null);
    setChannelError(null);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
      navOpen: false,
    }));
    clearThreadContext();
    syncRoute('replace', {
      activeTopic: topicId,
      composeTarget: {
        kind: 'private_channel',
        channel_id: channelId,
      },
      primarySection: 'timeline',
      timelineScope: {
        kind: 'channel',
        channel_id: channelId,
      },
    });
    await loadTopics(nextTopics, topicId, null);
  }

  async function handleJoinChannelAccess(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!inviteTokenInput.trim()) {
      setChannelError(translate('channels:errors.inviteTokenRequired'));
      return;
    }
    setChannelActionPending('join');
    try {
      const preview = await api.importChannelAccessToken(inviteTokenInput.trim());
      await activateImportedPrivateChannel(
        preview.topic_id,
        preview.channel_id,
        joinedChannelFromAccessTokenPreview(preview)
      );
    } catch (joinError) {
      setChannelError(messageFromError(joinError, translate('channels:errors.failedJoinChannel')));
    } finally {
      setChannelActionPending(null);
    }
  }

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmedComposer = composer.trim();
    const attachments = draftMediaItems.flatMap((item) => item.attachments);
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

      try {
        await api.createRepost(activeTopic, sourceTopic, repostTarget.object_id, trimmedComposer);
        releaseAllDraftPreviews();
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
        await loadTopics(trackedTopics, activeTopic, null);
        syncRoute('replace', {
          primarySection: 'timeline',
          selectedThread: null,
        });
      } catch (publishError) {
        setComposerError(
          publishError instanceof Error
            ? publishError.message
            : translate('common:errors.failedToPublish')
        );
      }
      return;
    }

    if (!trimmedComposer && attachments.length === 0) {
      return;
    }

    try {
      await api.createPost(
        activeTopic,
        trimmedComposer,
        replyTarget?.object_id ?? null,
        attachments,
        activeComposeChannel
      );
      releaseAllDraftPreviews();
      setComposer('');
      setDraftMediaItems([]);
      setAttachmentInputKey((value) => value + 1);
      setComposerError(null);
      setComposeDialogOpen(false);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
      }));
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      setReplyTarget(null);
      setRepostTarget(null);
      syncRoute('replace', {
        primarySection: 'timeline',
      });
    } catch (publishError) {
      setComposerError(
        publishError instanceof Error
          ? publishError.message
          : translate('common:errors.failedToPublish')
      );
    }
  }

  async function handleAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) {
      return;
    }

    const nextItems: DraftMediaItem[] = [];
    const failures: string[] = [];

    for (const file of files) {
      try {
        if (file.type.startsWith('image/')) {
          nextItems.push(await buildImageDraftItem(file));
          continue;
        }
        if (file.type.startsWith('video/')) {
          nextItems.push(await buildVideoDraftItem(file));
          continue;
        }
        failures.push(translate('common:errors.unsupportedAttachmentType', { name: file.name }));
      } catch (attachmentError) {
        failures.push(
          attachmentError instanceof Error
            ? attachmentError.message
            : translate('common:errors.failedToGenerateVideoPoster')
        );
      }
    }

    if (nextItems.length > 0) {
      nextItems.forEach(rememberDraftPreview);
      setDraftMediaItems((current) => [...current, ...nextItems]);
    }

    setComposerError(failures.length > 0 ? failures[0] : null);
    setAttachmentInputKey((value) => value + 1);
  }

  function handleRemoveDraftAttachment(itemId: string) {
    releaseDraftPreview(itemId);
    setDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

  async function handleDirectMessageAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }

    try {
      const nextItem = file.type.startsWith('image/')
        ? await buildImageDraftItem(file)
        : file.type.startsWith('video/')
          ? await buildVideoDraftItem(file)
          : null;
      if (!nextItem) {
        setDirectMessageError(
          translate('common:errors.unsupportedAttachmentType', { name: file.name })
        );
      } else {
        releaseAllDirectMessageDraftPreviews();
        rememberDirectMessageDraftPreview(nextItem);
        setDirectMessageDraftMediaItems([nextItem]);
        setDirectMessageError(null);
      }
    } catch (attachmentError) {
      setDirectMessageError(
        messageFromError(attachmentError, translate('common:errors.failedToGenerateVideoPoster'))
      );
    } finally {
      setDirectMessageAttachmentInputKey((value) => value + 1);
    }
  }

  function handleRemoveDirectMessageDraftAttachment(itemId: string) {
    releaseDirectMessageDraftPreview(itemId);
    setDirectMessageDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

  async function handleSendDirectMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedDirectMessagePeerPubkey) {
      return;
    }
    const trimmedComposer = directMessageComposer.trim();
    const attachments = directMessageDraftMediaItems.flatMap((item) => item.attachments);
    if (!trimmedComposer && attachments.length === 0) {
      return;
    }
    setDirectMessageSending(true);
    try {
      const messageId = await api.sendDirectMessage(
        selectedDirectMessagePeerPubkey,
        trimmedComposer || null,
        attachments,
        null
      );
      const existingConversation =
        state.directMessages.find(
          (conversation) => conversation.peer_pubkey === selectedDirectMessagePeerPubkey
        ) ?? null;
      const existingStatus =
        existingConversation?.status ??
        state.directMessageStatusByPeer[selectedDirectMessagePeerPubkey] ??
        null;
      const knownPeerAuthor =
        state.knownAuthorsByPubkey[selectedDirectMessagePeerPubkey] ?? selectedAuthor ?? null;
      const createdAt = Date.now();
      const optimisticAttachments: AttachmentView[] = attachments.map((attachment, index) => ({
        hash: `${messageId}-attachment-${index}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }));
      const optimisticMessage = {
        dm_id:
          existingConversation?.dm_id ??
          existingStatus?.dm_id ??
          [state.syncStatus.local_author_pubkey, selectedDirectMessagePeerPubkey].sort().join(':'),
        message_id: messageId,
        sender_pubkey: state.syncStatus.local_author_pubkey,
        recipient_pubkey: selectedDirectMessagePeerPubkey,
        created_at: createdAt,
        text: trimmedComposer,
        reply_to_message_id: null,
        attachments: optimisticAttachments,
        outgoing: true,
        delivered: true,
      } satisfies DirectMessageMessageView;
      const optimisticConversation = {
        dm_id: optimisticMessage.dm_id,
        peer_pubkey: selectedDirectMessagePeerPubkey,
        peer_name: knownPeerAuthor?.name ?? existingConversation?.peer_name ?? null,
        peer_display_name:
          knownPeerAuthor?.display_name ?? existingConversation?.peer_display_name ?? null,
        peer_picture: knownPeerAuthor?.picture ?? existingConversation?.peer_picture ?? null,
        peer_picture_asset:
          knownPeerAuthor?.picture_asset ?? existingConversation?.peer_picture_asset ?? null,
        updated_at: createdAt,
        last_message_at: createdAt,
        last_message_id: messageId,
        last_message_preview:
          trimmedComposer || directMessagePreviewFromAttachments(optimisticAttachments),
        status:
          existingStatus ??
          existingConversation?.status ?? {
            peer_pubkey: selectedDirectMessagePeerPubkey,
            dm_id: optimisticMessage.dm_id,
            mutual: true,
            send_enabled: true,
            peer_count: 1,
            pending_outbox_count: 0,
          },
      } satisfies DirectMessageConversationView;
      setDirectMessageTimelineByPeer((current) => ({
        ...current,
        [selectedDirectMessagePeerPubkey]: [
          optimisticMessage,
          ...(current[selectedDirectMessagePeerPubkey] ?? []).filter(
            (message) => message.message_id !== messageId
          ),
        ],
      }));
      setDirectMessages((current) => {
        const remaining = current.filter(
          (conversation) => conversation.peer_pubkey !== selectedDirectMessagePeerPubkey
        );
        return [optimisticConversation, ...remaining];
      });
      releaseAllDirectMessageDraftPreviews();
      setDirectMessageComposer('');
      setDirectMessageDraftMediaItems([]);
      setDirectMessageAttachmentInputKey((value) => value + 1);
      setDirectMessageError(null);
      await openDirectMessagePane(selectedDirectMessagePeerPubkey, { historyMode: 'replace' });
    } catch (sendError) {
      setDirectMessageError(messageFromError(sendError, 'failed to send direct message'));
    } finally {
      setDirectMessageSending(false);
    }
  }

  async function handleDeleteDirectMessageMessage(peerPubkey: string, messageId: string) {
    try {
      await api.deleteDirectMessageMessage(peerPubkey, messageId);
      await openDirectMessagePane(peerPubkey, { historyMode: 'replace' });
    } catch (deleteError) {
      setDirectMessageError(messageFromError(deleteError, 'failed to delete direct message'));
    }
  }

  async function handleClearDirectMessage(peerPubkey: string) {
    try {
      await api.clearDirectMessage(peerPubkey);
      await openDirectMessagePane(peerPubkey, { historyMode: 'replace' });
    } catch (clearError) {
      setDirectMessageError(messageFromError(clearError, 'failed to clear direct message'));
    }
  }

  function patchReactionState(reactionState: Parameters<typeof patchReactionStateIntoPosts>[1]) {
    setTimelinesByTopic((current) =>
      Object.fromEntries(
        Object.entries(current).map(([topic, posts]) => [
          topic,
          patchReactionStateIntoPosts(posts, reactionState),
        ])
      )
    );
    setPublicTimelinesByTopic((current) =>
      Object.fromEntries(
        Object.entries(current).map(([topic, posts]) => [
          topic,
          patchReactionStateIntoPosts(posts, reactionState),
        ])
      )
    );
    setThread((current) => patchReactionStateIntoPosts(current, reactionState));
    setProfileTimeline((current) => patchReactionStateIntoPosts(current, reactionState));
    setSelectedAuthorTimeline((current) => patchReactionStateIntoPosts(current, reactionState));
  }

  async function handleToggleReaction(post: PostView, reactionKey: ReactionKeyInput) {
    const topicId = publishedTopicIdForPost(post);
    if (!topicId) {
      setError(translate('common:errors.failedToPublish'));
      return;
    }
    try {
      const nextState = await api.toggleReaction(
        topicId,
        post.object_id,
        reactionKey,
        post.channel_id ? { kind: 'private_channel', channel_id: post.channel_id } : { kind: 'public' }
      );
      patchReactionState(nextState);
      try {
        setRecentReactions(await api.listRecentReactions(8));
      } catch {
        // Keep the toggled state even if the quick-reaction history refresh misses.
      }
      setError(null);
    } catch (reactionError) {
      setError(messageFromError(reactionError, translate('common:errors.failedToPublish')));
    }
  }

  async function handleCreateCustomReactionAsset(
    file: File,
    cropRect: CustomReactionCropRect,
    searchKey: string
  ) {
    setReactionCreatePending(true);
    try {
      const upload = await fileToCreateAttachment(file, 'image_original');
      const asset = await api.createCustomReactionAsset(upload, cropRect, searchKey);
      setOwnedReactionAssets((current) => [
        asset,
        ...current.filter((item) => item.asset_id !== asset.asset_id),
      ]);
      setReactionPanelState({
        status: 'ready',
        error: null,
      });
    } catch (reactionError) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(reactionError, translate('common:errors.failedToPublish')),
      });
    } finally {
      setReactionCreatePending(false);
    }
  }

  async function handleBookmarkCustomReaction(asset: Parameters<DesktopApi['bookmarkCustomReaction']>[0]) {
    try {
      const bookmarked = await api.bookmarkCustomReaction(asset);
      setBookmarkedReactionAssets((current) => [
        bookmarked,
        ...current.filter((item) => item.asset_id !== bookmarked.asset_id),
      ]);
      setReactionPanelState({
        status: 'ready',
        error: null,
      });
    } catch (bookmarkError) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(bookmarkError, translate('common:errors.failedToPublish')),
      });
    }
  }

  async function handleRemoveBookmarkedCustomReaction(assetId: string) {
    try {
      await api.removeBookmarkedCustomReaction(assetId);
      setBookmarkedReactionAssets((current) => current.filter((item) => item.asset_id !== assetId));
      setReactionPanelState({
        status: 'ready',
        error: null,
      });
    } catch (bookmarkError) {
      setReactionPanelState({
        status: 'error',
        error: messageFromError(bookmarkError, translate('common:errors.failedToPublish')),
      });
    }
  }

  async function handleToggleBookmarkedPost(post: PostView) {
    const topicId = publishedTopicIdForPost(post);
    if (!topicId) {
      setError(translate('common:errors.failedToUpdateBookmark'));
      return;
    }
    try {
      if (bookmarkedPostIds.has(post.object_id)) {
        await api.removeBookmarkedPost(post.object_id);
        setBookmarkedPosts((current) =>
          current.filter((item) => item.post.object_id !== post.object_id)
        );
      } else {
        const bookmarked = await api.bookmarkPost(topicId, post.object_id);
        setBookmarkedPosts((current) => [
          bookmarked,
          ...current.filter((item) => item.post.object_id !== bookmarked.post.object_id),
        ]);
      }
      setError(null);
    } catch (bookmarkError) {
      setError(messageFromError(bookmarkError, translate('common:errors.failedToUpdateBookmark')));
    }
  }

  function beginReply(post: PostView) {
    const threadId = post.root_id ?? post.object_id;
    setRepostTarget(null);
    setReplyTarget(post);
    setComposeDialogOpen(true);
    setSelectedThread(threadId);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('push', {
      selectedThread: threadId,
      selectedAuthorPubkey: null,
    });
    void openThread(threadId, { historyMode: 'replace' });
  }

  function clearReply() {
    setReplyTarget(null);
    setRepostTarget(null);
  }

  function clearRepost() {
    setRepostTarget(null);
  }

  function openNewPostDialog() {
    clearReply();
    clearRepost();
    setComposeDialogOpen(true);
  }

  function openFloatingActionDialog() {
    if (shellChromeState.activePrimarySection === 'live') {
      setLiveCreateDialogOpen(true);
      return;
    }
    if (shellChromeState.activePrimarySection === 'game') {
      setGameCreateDialogOpen(true);
      return;
    }
    openNewPostDialog();
  }

  async function handleSimpleRepost(post: PostView) {
    const sourceTopic = publishedTopicIdForPost(post);
    if (!sourceTopic || !canCreateRepostFromPost(post)) {
      setComposerError(translate('common:errors.failedToPublish'));
      return;
    }

    try {
      await api.createRepost(activeTopic, sourceTopic, post.object_id, null);
      setComposerError(null);
      setReplyTarget(null);
      setRepostTarget(null);
      setSelectedThread(null);
      setThread([]);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'timeline',
      }));
      await loadTopics(trackedTopics, activeTopic, null);
      syncRoute('replace', {
        primarySection: 'timeline',
        selectedThread: null,
      });
    } catch (repostError) {
      setComposerError(
        repostError instanceof Error
          ? repostError.message
          : translate('common:errors.failedToPublish')
      );
    }
  }

  function beginQuoteRepost(post: PostView) {
    if (!canCreateRepostFromPost(post)) {
      return;
    }
    releaseAllDraftPreviews();
    setDraftMediaItems([]);
    setAttachmentInputKey((value) => value + 1);
    setComposer('');
    setComposerError(null);
    setReplyTarget(null);
    setRepostTarget(post);
    setComposeDialogOpen(true);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('replace', {
      selectedAuthorPubkey: null,
    });
  }

  async function handleRelationshipAction(authorPubkey: string, following: boolean) {
    try {
      const nextView = following
        ? await api.unfollowAuthor(authorPubkey)
        : await api.followAuthor(authorPubkey);
      setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [nextView]));
      if (selectedAuthorPubkey === authorPubkey) {
        setSelectedAuthor(nextView);
        setAuthorError(null);
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (relationshipError) {
      setAuthorError(
        relationshipError instanceof Error
          ? relationshipError.message
          : translate('common:errors.failedToUpdateFollowState')
      );
    }
  }

  async function handleMuteAction(authorPubkey: string, muted: boolean) {
    try {
      const nextView = muted
        ? await api.unmuteAuthor(authorPubkey)
        : await api.muteAuthor(authorPubkey);
      setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [nextView]));
      if (selectedAuthorPubkey === authorPubkey) {
        setSelectedAuthor(nextView);
        setAuthorError(null);
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (muteError) {
      setAuthorError(
        muteError instanceof Error
          ? muteError.message
          : translate('common:errors.failedToUpdateMuteState')
      );
    }
  }

  async function handleSaveDiscoverySeeds() {
    try {
      const seedEntries = discoverySeedInput
        .split('\n')
        .map((entry) => entry.trim())
        .filter(Boolean);
      const nextConfig = await api.setDiscoverySeeds(seedEntries);
      setDiscoveryConfig(nextConfig);
      setDiscoverySeedInput(seedPeersToEditorValue(nextConfig));
      setDiscoveryEditorDirty(false);
      setDiscoveryError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace');
    } catch (saveError) {
      setDiscoveryError(
        saveError instanceof Error
          ? saveError.message
          : translate('common:errors.failedToUpdateDiscoverySeeds')
      );
    }
  }

  async function handleSaveCommunityNodes() {
    try {
      const baseUrls = communityNodeInput
        .split('\n')
        .map((entry) => entry.trim())
        .filter(Boolean);
      const nextConfig = await api.setCommunityNodeConfig(baseUrls);
      setCommunityNodeConfig(nextConfig);
      setCommunityNodeInput(communityNodesToEditorValue(nextConfig));
      setCommunityNodeEditorDirty(false);
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace');
    } catch (saveError) {
      setCommunityNodeError(
        saveError instanceof Error
          ? saveError.message
          : translate('common:errors.failedToUpdateCommunityNodes')
      );
    }
  }

  async function handleClearCommunityNodes() {
    try {
      await api.clearCommunityNodeConfig();
      setCommunityNodeConfig(DEFAULT_COMMUNITY_NODE_CONFIG);
      setCommunityNodeStatuses([]);
      setCommunityNodeInput('');
      setCommunityNodeEditorDirty(false);
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      syncRoute('replace');
    } catch (clearError) {
      setCommunityNodeError(
        clearError instanceof Error
          ? clearError.message
          : translate('common:errors.failedToClearCommunityNodes')
      );
    }
  }

  async function handleAuthenticateCommunityNode(baseUrl: string) {
    try {
      const nextStatus = await api.authenticateCommunityNode(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (authError) {
      setCommunityNodeError(
        authError instanceof Error
          ? authError.message
          : translate('common:errors.failedToAuthenticateCommunityNode')
      );
    }
  }

  async function handleClearCommunityNodeToken(baseUrl: string) {
    try {
      const nextStatus = await api.clearCommunityNodeToken(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (clearError) {
      setCommunityNodeError(
        clearError instanceof Error
          ? clearError.message
          : translate('common:errors.failedToClearCommunityNodeToken')
      );
    }
  }

  async function handleRefreshCommunityNode(baseUrl: string) {
    try {
      const nextStatus = await api.refreshCommunityNodeMetadata(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (refreshError) {
      setCommunityNodeError(
        refreshError instanceof Error
          ? refreshError.message
          : translate('common:errors.failedToRefreshCommunityNode')
      );
    }
  }

  async function handleFetchCommunityNodeConsents(baseUrl: string) {
    try {
      const nextStatus = await api.getCommunityNodeConsentStatus(baseUrl);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (consentError) {
      setCommunityNodeError(
        consentError instanceof Error
          ? consentError.message
          : translate('common:errors.failedToFetchConsentStatus')
      );
    }
  }

  async function handleAcceptCommunityNodeConsents(baseUrl: string) {
    try {
      const nextStatus = await api.acceptCommunityNodeConsents(baseUrl, []);
      setCommunityNodeStatuses((current) => upsertCommunityNodeStatus(current, nextStatus));
      setCommunityNodeConfig((current) => syncCommunityNodeConfigWithStatus(current, nextStatus));
      setCommunityNodeError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (consentError) {
      setCommunityNodeError(
        consentError instanceof Error
          ? consentError.message
          : translate('common:errors.failedToAcceptConsents')
      );
    }
  }

  async function handleImportPeer() {
    if (!peerTicket.trim()) {
      return;
    }
    try {
      await api.importPeerTicket(peerTicket.trim());
      setPeerTicket('');
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (importError) {
      setError(
        importError instanceof Error
          ? importError.message
          : translate('common:errors.failedToImportPeer')
      );
    }
  }

  async function handleCreateLiveSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!liveTitle.trim()) {
      setLiveError(translate('live:errors.titleRequired'));
      return;
    }
    setLiveCreatePending(true);
    try {
      await api.createLiveSession(
        activeTopic,
        liveTitle.trim(),
        liveDescription.trim(),
        activeComposeChannel
      );
      setLiveTitle('');
      setLiveDescription('');
      setLiveError(null);
      setLiveCreateDialogOpen(false);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'live',
      }));
      syncRoute('replace', {
        primarySection: 'live',
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (liveCreateError) {
      setLiveError(messageFromError(liveCreateError, translate('live:errors.failedCreate')));
    } finally {
      setLiveCreatePending(false);
    }
  }

  async function handleJoinLiveSession(sessionId: string) {
    setLivePendingBySessionId((current) => ({
      ...current,
      [sessionId]: true,
    }));
    try {
      await api.joinLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (joinError) {
      setLiveError(messageFromError(joinError, translate('live:errors.failedJoin')));
    } finally {
      setLivePendingBySessionId((current) => {
        const next = { ...current };
        delete next[sessionId];
        return next;
      });
    }
  }

  async function handleLeaveLiveSession(sessionId: string) {
    setLivePendingBySessionId((current) => ({
      ...current,
      [sessionId]: true,
    }));
    try {
      await api.leaveLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (leaveError) {
      setLiveError(messageFromError(leaveError, translate('live:errors.failedLeave')));
    } finally {
      setLivePendingBySessionId((current) => {
        const next = { ...current };
        delete next[sessionId];
        return next;
      });
    }
  }

  async function handleEndLiveSession(sessionId: string) {
    setLivePendingBySessionId((current) => ({
      ...current,
      [sessionId]: true,
    }));
    try {
      await api.endLiveSession(activeTopic, sessionId);
      setLiveError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (endError) {
      setLiveError(messageFromError(endError, translate('live:errors.failedEnd')));
    } finally {
      setLivePendingBySessionId((current) => {
        const next = { ...current };
        delete next[sessionId];
        return next;
      });
    }
  }

  async function handleCreateGameRoom(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const participants = Array.from(
      new Set(
        gameParticipantsInput
          .split(',')
          .map((value) => value.trim())
          .filter((value) => value.length > 0)
      )
    );
    if (!gameTitle.trim()) {
      setGameError(translate('game:errors.titleRequired'));
      return;
    }
    if (participants.length < 2) {
      setGameError(translate('game:errors.participantsRequired'));
      return;
    }
    setGameCreatePending(true);
    try {
      await api.createGameRoom(
        activeTopic,
        gameTitle.trim(),
        gameDescription.trim(),
        participants,
        activeComposeChannel
      );
      setGameTitle('');
      setGameDescription('');
      setGameParticipantsInput('');
      setGameError(null);
      setGameCreateDialogOpen(false);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'game',
      }));
      syncRoute('replace', {
        primarySection: 'game',
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (createError) {
      setGameError(messageFromError(createError, translate('game:errors.failedCreate')));
    } finally {
      setGameCreatePending(false);
    }
  }

  function updateGameDraft(roomId: string, update: (draft: GameEditorDraft) => GameEditorDraft) {
    setGameDrafts((current) => {
      const existingRoom = activeGameRooms.find((room) => room.room_id === roomId);
      const draft = current[roomId] ?? (existingRoom ? createGameEditorDraft(existingRoom) : null);
      if (!draft) {
        return current;
      }
      return {
        ...current,
        [roomId]: update(draft),
      };
    });
  }

  async function handleUpdateGameRoom(roomId: string) {
    const room = activeGameRooms.find((candidate) => candidate.room_id === roomId);
    if (!room) {
      return;
    }
    const draft = gameDrafts[room.room_id] ?? createGameEditorDraft(room);
    const scores: GameScoreView[] = [];
    for (const score of room.scores) {
      const rawScore = draft.scores[score.participant_id] ?? String(score.score);
      const parsed = Number.parseInt(rawScore, 10);
      if (Number.isNaN(parsed)) {
        setGameError(translate('game:errors.invalidScore', { label: score.label }));
        return;
      }
      scores.push({
        participant_id: score.participant_id,
        label: score.label,
        score: parsed,
      });
    }
    setGameSavingByRoomId((current) => ({
      ...current,
      [room.room_id]: true,
    }));
    try {
      await api.updateGameRoom(
        activeTopic,
        room.room_id,
        draft.status,
        draft.phase_label.trim() || null,
        scores
      );
      setGameError(null);
      setGameDrafts((current) => {
        const next = { ...current };
        delete next[room.room_id];
        return next;
      });
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (updateError) {
      setGameError(messageFromError(updateError, translate('game:errors.failedUpdate')));
    } finally {
      setGameSavingByRoomId((current) => {
        const next = { ...current };
        delete next[room.room_id];
        return next;
      });
    }
  }

  return {
    handleProfileFieldChange,
    handleProfileAvatarSelection,
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
    handlePublish,
    handleAttachmentSelection,
    handleRemoveDraftAttachment,
    handleDirectMessageAttachmentSelection,
    handleRemoveDirectMessageDraftAttachment,
    handleSendDirectMessage,
    handleDeleteDirectMessageMessage,
    handleClearDirectMessage,
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
