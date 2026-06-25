import type { FormEvent } from 'react';

import type {
  JoinedPrivateChannelView,
  ProfileInput,
} from '@/lib/api';
import { fileToCreateAttachment } from '@/lib/attachments';

import {
  DEFAULT_COMMUNITY_NODE_CONFIG,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
} from '@/shell/store';
import {
  communityNodeDraftNodesToConfigInput,
  communityNodesToDraftNodes,
  joinedChannelFromAccessTokenPreview,
  messageFromError,
  privateComposeTarget,
  privateTimelineScope,
  profileInputFromProfile,
  seedPeersToEditorValue,
  syncCommunityNodeConfigWithStatus,
  upsertCommunityNodeStatus,
  upsertJoinedChannel,
} from '@/shell/selectors';

import type {
  ActionsBaseParams,
  NullableStringDispatch,
  NumberStateDispatch,
  Setter,
} from './shared';

type ProfileTopicChannelParams = ActionsBaseParams & {
  activePrivateChannel: JoinedPrivateChannelView | null;
  activeTopic: string;
  channelAudienceInput: 'invite_only' | 'friend_only' | 'friend_plus';
  channelLabelInput: string;
  communityNodeInput: Array<{ id: string; base_url: string; auto_approve: boolean }>;
  discoverySeedInput: string;
  inviteTokenInput: string;
  localProfile: {
    pubkey: string;
    name?: string | null;
    display_name?: string | null;
    about?: string | null;
    picture?: string | null;
    picture_asset?: { hash: string; mime: string; bytes: number; role: 'profile_avatar' } | null;
    updated_at: number;
  } | null;
  profileDraft: ProfileInput;
  selectedChannelIdByTopic: Record<string, string | null>;
  selectedThread: string | null;
  topicInput: string;
  trackedTopics: string[];
  clearThreadContext: () => void;
  setProfileAvatarPreviewUrl: NullableStringDispatch;
  setProfileAvatarInputKey: NumberStateDispatch;
  setTrackedTopics: Setter<'trackedTopics'>;
  setActiveTopic: Setter<'activeTopic'>;
  setTopicInput: Setter<'topicInput'>;
  setTimelineScopeByTopic: Setter<'timelineScopeByTopic'>;
  setComposeChannelByTopic: Setter<'composeChannelByTopic'>;
  setSelectedChannelIdByTopic: Setter<'selectedChannelIdByTopic'>;
  setShellChromeState: Setter<'shellChromeState'>;
  setProfileDraft: Setter<'profileDraft'>;
  setProfileDirty: Setter<'profileDirty'>;
  setProfileError: Setter<'profileError'>;
  setProfilePanelState: Setter<'profilePanelState'>;
  setProfileSaving: Setter<'profileSaving'>;
  setLocalProfile: Setter<'localProfile'>;
  setChannelLabelInput: Setter<'channelLabelInput'>;
  setChannelAudienceInput: Setter<'channelAudienceInput'>;
  setInviteTokenInput: Setter<'inviteTokenInput'>;
  setInviteOutput: Setter<'inviteOutput'>;
  setInviteOutputLabel: Setter<'inviteOutputLabel'>;
  setChannelError: Setter<'channelError'>;
  setChannelPanelStateByTopic: Setter<'channelPanelStateByTopic'>;
  setChannelActionPending: Setter<'channelActionPending'>;
  setJoinedChannelsByTopic: Setter<'joinedChannelsByTopic'>;
  setCommunityNodeConfig: Setter<'communityNodeConfig'>;
  setCommunityNodeStatuses: Setter<'communityNodeStatuses'>;
  setCommunityNodeInput: Setter<'communityNodeInput'>;
  setCommunityNodeEditorDirty: Setter<'communityNodeEditorDirty'>;
  setCommunityNodeError: Setter<'communityNodeError'>;
  setDiscoveryConfig: Setter<'discoveryConfig'>;
  setDiscoverySeedInput: Setter<'discoverySeedInput'>;
  setDiscoveryEditorDirty: Setter<'discoveryEditorDirty'>;
  setDiscoveryError: Setter<'discoveryError'>;
};

export function createProfileTopicChannelActions({
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
}: ProfileTopicChannelParams) {
  function handleProfileFieldChange(field: 'displayName' | 'name' | 'about', value: string) {
    const nextField: keyof ProfileInput = field === 'displayName' ? 'display_name' : field;
    setProfileDraft((current) => ({
      ...current,
      [nextField]: value,
    }));
    setProfileDirty(true);
  }

  async function handleProfileAvatarFile(file: File) {
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

  async function handleToggleTopicGossip(topic: string, enabled: boolean) {
    await api.setTopicGossipEnabled(topic, enabled);
    await loadTopics(trackedTopics, activeTopic, selectedThread);
  }

  async function handleToggleChannelGossip(
    topic: string,
    channelId: string,
    enabled: boolean
  ) {
    await api.setChannelGossipEnabled(topic, channelId, enabled);
    await loadTopics(trackedTopics, activeTopic, selectedThread);
  }

  async function handleCreatePrivateChannel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!channelLabelInput.trim()) {
      setChannelError(translate('channels:errors.channelLabelRequired'));
      return;
    }
    setChannelActionPending('create');
    setInviteOutput(null);
    try {
      const channel = await api.createPrivateChannel(
        activeTopic,
        channelLabelInput.trim(),
        channelAudienceInput
      );
      let nextChannelError: string | null = null;
      try {
        const access = await api.exportChannelAccessToken(activeTopic, channel.channel_id, null);
        setInviteOutput(access.token);
        setInviteOutputLabel(access.kind);
      } catch (shareError) {
        nextChannelError = messageFromError(
          shareError,
          translate('channels:errors.failedShareChannel')
        );
      }
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
      setChannelError(nextChannelError);
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

  async function handleLeavePrivateChannel(topicId: string, channelId: string) {
    setChannelActionPending('leave');
    try {
      await api.leavePrivateChannel(topicId, channelId);
      setJoinedChannelsByTopic((current) => ({
        ...current,
        [topicId]: (current[topicId] ?? []).filter(
          (channel) => channel.channel_id !== channelId
        ),
      }));
      setChannelPanelStateByTopic((current) => ({
        ...current,
        [topicId]: {
          status: 'ready',
          error: null,
        },
      }));
      setInviteOutput(null);
      setChannelError(null);
      const leavingSelectedChannel = selectedChannelIdByTopic[topicId] === channelId;
      if (leavingSelectedChannel) {
        setSelectedChannelIdByTopic((current) => ({
          ...current,
          [topicId]: null,
        }));
        setTimelineScopeByTopic((current) => ({
          ...current,
          [topicId]: PUBLIC_TIMELINE_SCOPE,
        }));
        setComposeChannelByTopic((current) => ({
          ...current,
          [topicId]: PUBLIC_CHANNEL_REF,
        }));
        if (topicId === activeTopic) {
          syncRoute('replace', {
            activeTopic: topicId,
            composeTarget: PUBLIC_CHANNEL_REF,
            timelineScope: PUBLIC_TIMELINE_SCOPE,
          });
        }
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (leaveError) {
      setChannelError(messageFromError(leaveError, translate('channels:errors.failedLeaveChannel')));
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
    await handleImportChannelAccessToken(inviteTokenInput.trim());
  }

  async function handleImportChannelAccessToken(token: string) {
    setChannelActionPending('join');
    try {
      const preview = await api.importChannelAccessToken(token.trim());
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
      const nextConfig = await api.setCommunityNodeConfig(
        communityNodeDraftNodesToConfigInput(communityNodeInput)
      );
      setCommunityNodeConfig(nextConfig);
      setCommunityNodeInput(communityNodesToDraftNodes(nextConfig));
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
      setCommunityNodeInput([]);
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
      throw consentError;
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
      throw consentError;
    }
  }

  return {
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
    handleClearCommunityNodeToken,
    handleRefreshCommunityNode,
    handleFetchCommunityNodeConsents,
    handleAcceptCommunityNodeConsents,
  };
}
