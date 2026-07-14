import {
  useMemo,
} from 'react';

import type {
  ComposerDraftMediaView,
  MentionCandidate,
  ThreadPanelState,
  TopicDiagnosticSummary,
} from '@/components/core/types';
import type { TimelineWorkspaceView } from '@/components/shell/types';
import type {
  GameRoomView,
  JoinedPrivateChannelView,
  LiveSessionView,
  PostView,
  TopicSyncStatus,
} from '@/lib/api';
import type { DesktopTheme } from '@/lib/theme';
import { useChannelsViewModels } from '@/shell/viewModels/useChannelsViewModels';
import { useMessagesViewModels } from '@/shell/viewModels/useMessagesViewModels';
import { useProfileViewModels } from '@/shell/viewModels/useProfileViewModels';
import { useSettingsViewModels } from '@/shell/viewModels/useSettingsViewModels';
import { useTimelineViewModels } from '@/shell/viewModels/useTimelineViewModels';

import { useShallow } from 'zustand/react/shallow';
import {
  PRIMARY_SECTION_ITEMS,
  SETTINGS_SECTION_COPY,
} from '@/shell/routes';
import {
  type SupportedLocale,
} from '@/i18n';
import {
  activeTimelineStorageKey,
  DEFAULT_ASYNC_PANEL_STATE,
  useDesktopShellStore,
} from '@/shell/store';
import {
  audienceLabelForChannelRef,
  authorDisplayLabel,
  formatBytes,
  formatCount,
  formatLastReceivedLabel,
  resolveProfilePictureSrc,
  topicConnectionLabel,
} from '@/shell/presentation';
import { selectShellViewModelsSlice } from '@/shell/storeSelectors';

type UseDesktopShellViewModelsArgs = {
  t: (key: string, options?: Record<string, unknown>) => string;
  translate: (key: string, options?: Record<string, unknown>) => string;
  locale: SupportedLocale;
  theme: DesktopTheme;
  profileAvatarPreviewUrl: string | null;
};

const EMPTY_POSTS: PostView[] = [];
const EMPTY_LIVE_SESSIONS: LiveSessionView[] = [];
const EMPTY_GAME_ROOMS: GameRoomView[] = [];
const EMPTY_JOINED_CHANNELS: JoinedPrivateChannelView[] = [];

export function useDesktopShellViewModels({
  t,
  translate,
  locale,
  theme,
  profileAvatarPreviewUrl,
}: UseDesktopShellViewModelsArgs) {
  const state = useDesktopShellStore(useShallow(selectShellViewModelsSlice));
  const activeTopic = state.activeTopic;
  const selectedPrivateChannelId = state.selectedChannelIdByTopic[activeTopic] ?? null;
  const activeJoinedChannels = state.joinedChannelsByTopic[activeTopic] ?? EMPTY_JOINED_CHANNELS;
  const activeTimeline = state.timelinesByKey[activeTimelineStorageKey(state, activeTopic)] ?? EMPTY_POSTS;
  const activeLiveSessions = state.liveSessionsByTopic[activeTopic] ?? EMPTY_LIVE_SESSIONS;
  const activeGameRooms = state.gameRoomsByTopic[activeTopic] ?? EMPTY_GAME_ROOMS;
  const activeTimelineScope = state.timelineScopeByTopic[activeTopic] ?? { kind: 'public' as const };
  const activeComposeChannel = state.repostTarget
    ? { kind: 'public' as const }
    : state.replyTarget?.channel_id
      ? {
          kind: 'private_channel' as const,
          channel_id: state.replyTarget.channel_id,
        }
      : state.composeChannelByTopic[activeTopic] ?? { kind: 'public' as const };
  const activeComposeAudienceLabel = state.repostTarget
    ? translate('common:audience.public')
    : state.replyTarget
      ? state.replyTarget.audience_label
      : audienceLabelForChannelRef(activeComposeChannel, activeJoinedChannels);
  const activePrivateChannel =
    selectedPrivateChannelId
      ? activeJoinedChannels.find((channel) => channel.channel_id === selectedPrivateChannelId) ?? null
      : null;
  const activeSocialConnections =
    state.socialConnections[state.shellChromeState.profileConnectionsView];
  const activeSocialConnectionViews =
    state.socialConnections[state.shellChromeState.profileConnectionsView] ?? [];
  const activeChannelPanelState =
    state.channelPanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE;
  const activeLivePanelState =
    state.livePanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE;
  const activeGamePanelState =
    state.gamePanelStateByTopic[activeTopic] ?? DEFAULT_ASYNC_PANEL_STATE;
  const topicDiagnostics = Object.fromEntries(
    state.syncStatus.topic_diagnostics.map((diagnostic) => [diagnostic.topic, diagnostic])
  ) as Record<string, TopicSyncStatus>;
  const {
    syncStatus,
    livePendingBySessionId,
    gameDrafts,
    profileDraft,
    localProfile,
    mediaObjectUrls,
    communityNodeStatuses,
    thread,
    trackedTopics,
    joinedChannelsByTopic,
    selectedChannelIdByTopic,
    draftMediaItems,
    directMessageDraftMediaItems,
    selectedThread,
    selectedAuthor,
    knownAuthorsByPubkey,
    authorError,
    error,
    localPeerTicket,
    peerTicket,
    discoveryConfig,
    discoveryError,
    discoverySeedInput,
    discoveryEditorDirty,
    communityNodeConfig,
    communityNodeError,
    communityNodeInput,
    communityNodeManifests,
    communityNodeEditorDirty,
    reactionPanelState,
    ownedReactionAssets,
    bookmarkedReactionAssets,
    bookmarkedPosts,
    profileTimeline,
    selectedAuthorTimeline,
    replyTarget,
    repostTarget,
    unsupportedVideoManifests,
    directMessages,
    shellChromeState,
  } = state;
  const floatingActionLabel = useMemo(() => {
    if (shellChromeState.activePrimarySection === 'live') {
      return t('live:actions.start');
    }
    if (shellChromeState.activePrimarySection === 'game') {
      return t('game:actions.createRoom');
    }
    return t('common:actions.publish');
  }, [shellChromeState.activePrimarySection, t]);

  const showFloatingActionButton =
    shellChromeState.activePrimarySection !== 'profile' &&
    shellChromeState.activePrimarySection !== 'messages' &&
    shellChromeState.activePrimarySection !== 'notifications' &&
    !(
      shellChromeState.activePrimarySection === 'timeline' &&
      shellChromeState.timelineView === 'bookmarks'
    );

  const {
    activeTimelinePostViews,
    bookmarkedTimelinePostViews,
    composerSourcePreview,
    profileTimelinePostViews,
    selectedAuthorTimelinePostViews,
    threadPostViews,
  } = useTimelineViewModels({
    activeJoinedChannels,
    activeTimeline,
    bookmarkedPosts,
    knownAuthorsByPubkey,
    localAuthorPubkey: syncStatus.local_author_pubkey,
    locale,
    localProfile,
    mediaObjectUrls,
    profileTimeline,
    replyTarget,
    repostTarget,
    selectedAuthorTimeline,
    thread,
    unsupportedVideoManifests,
  });

  const {
    channelAudienceOptions,
    privateChannelListItems,
    liveSessionListItems,
    gameDraftViews,
  } = useChannelsViewModels({
    activeGameRooms,
    activeJoinedChannels,
    activeLiveSessions,
    gameDrafts,
    livePendingBySessionId,
    localAuthorPubkey: syncStatus.local_author_pubkey,
    selectedPrivateChannelId,
    t,
  });

  const {
    profileEditorFields,
    profileEditorPictureSrc,
    profileEditorHasPicture,
    authorDetailView,
  } = useProfileViewModels({
    authorError,
    knownAuthorsByPubkey,
    localAuthorPubkey: syncStatus.local_author_pubkey,
    localProfile,
    mediaObjectUrls,
    profileAvatarPreviewUrl,
    profileDraft,
    selectedAuthor,
    t,
  });

  const {
    selectedDirectMessageConversation,
    selectedDirectMessageTimeline,
    selectedDirectMessageStatus,
    selectedDirectMessagePeerAuthor,
    selectedDirectMessagePeerLabel,
    selectedDirectMessagePeerPicture,
    localDirectMessageAuthorPicture,
    directMessageDraftViews,
  } = useMessagesViewModels({
    directMessageDraftMediaItems,
    directMessages,
    directMessageStatusByPeer: state.directMessageStatusByPeer,
    directMessageTimelineByPeer: state.directMessageTimelineByPeer,
    knownAuthorsByPubkey,
    locale,
    localProfile,
    mediaObjectUrls,
    selectedDirectMessagePeerPubkey: state.selectedDirectMessagePeerPubkey,
    translate,
  });

  const {
    communityNodeStatusByBaseUrl,
    effectivePeerIds,
    connectivityPanelView,
    appearancePanelView,
    discoveryPanelView,
    communityNodePanelView,
    reactionsPanelView,
  } = useSettingsViewModels({
    bookmarkedReactionAssets,
    communityNodeConfig,
    communityNodeEditorDirty,
    communityNodeError,
    communityNodeInput,
    communityNodeManifests,
    communityNodeStatuses,
    discoveryConfig,
    discoveryEditorDirty,
    discoveryError,
    discoverySeedInput,
    error,
    locale,
    localPeerTicket,
    ownedReactionAssets,
    peerTicket,
    reactionPanelState,
    syncStatus,
    t,
    theme,
    topicDiagnostics,
    trackedTopics,
  });

  const gossipDisabledTopics = useMemo(
    () => new Set(syncStatus.gossip_disabled_topics ?? []),
    [syncStatus.gossip_disabled_topics]
  );
  const gossipDisabledChannels = useMemo(
    () => new Set(syncStatus.gossip_disabled_channels ?? []),
    [syncStatus.gossip_disabled_channels]
  );
  const topicNavItems = useMemo<TopicDiagnosticSummary[]>(
    () =>
      trackedTopics.map((topic) => ({
        topic,
        active: topic === activeTopic,
        publicActive: topic === activeTopic && (selectedChannelIdByTopic[topic] ?? null) === null,
        removable: trackedTopics.length > 1,
        connectionLabel: topicConnectionLabel(topicDiagnostics[topic]),
        peerCount: topicDiagnostics[topic]?.peer_count ?? 0,
        lastReceivedLabel: formatLastReceivedLabel(topicDiagnostics[topic]?.last_received_at, locale),
        lastReceivedAt: topicDiagnostics[topic]?.last_received_at ?? null,
        gossipJoined: !gossipDisabledTopics.has(topic),
        channels:
          topic === activeTopic
            ? (joinedChannelsByTopic[topic] ?? []).map((channel) => ({
                channelId: channel.channel_id,
                label: channel.label,
                audienceKind: channel.audience_kind,
                active: selectedChannelIdByTopic[topic] === channel.channel_id,
                gossipJoined:
                  !gossipDisabledTopics.has(topic) &&
                  !gossipDisabledChannels.has(`${topic}::${channel.channel_id}`),
              }))
            : [],
      })),
    [
      activeTopic,
      gossipDisabledChannels,
      gossipDisabledTopics,
      joinedChannelsByTopic,
      locale,
      selectedChannelIdByTopic,
      topicDiagnostics,
      trackedTopics,
    ]
  );
  const composerDraftViews = useMemo<ComposerDraftMediaView[]>(
    () =>
      draftMediaItems.map((item) => ({
        id: item.id,
        sourceName: item.source_name,
        previewUrl: item.preview_url,
        attachments: item.attachments.map((attachment) => ({
          key: `${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`,
          label: attachment.role ?? translate('common:fallbacks.attachment'),
          mime: attachment.mime,
          byteSizeLabel: formatBytes(attachment.byte_size, locale),
        })),
      })),
    [draftMediaItems, locale, translate]
  );
  const followingConnections = state.socialConnections.following;
  const mentionCandidates = useMemo<MentionCandidate[]>(() => {
    const seen = new Set<string>();
    const result: MentionCandidate[] = [];
    for (const view of [...followingConnections, ...Object.values(knownAuthorsByPubkey)]) {
      const pubkey = view.author_pubkey;
      if (!pubkey || pubkey === syncStatus.local_author_pubkey || seen.has(pubkey)) {
        continue;
      }
      seen.add(pubkey);
      result.push({
        pubkey,
        label: authorDisplayLabel(pubkey, view.display_name, view.name),
        displayName: view.display_name ?? null,
        name: view.name ?? null,
        about: view.about ?? null,
        picture: resolveProfilePictureSrc(view, mediaObjectUrls),
      });
    }
    return result;
  }, [followingConnections, knownAuthorsByPubkey, mediaObjectUrls, syncStatus.local_author_pubkey]);
  const threadPanelState = useMemo<ThreadPanelState>(
    () => ({
      selectedThreadId: selectedThread,
      summary: selectedThread
        ? t('shell:context.threadSummary', { count: formatCount(thread.length) })
        : t('shell:context.threadEmpty'),
      emptyCopy: t('shell:context.threadEmpty'),
    }),
    [selectedThread, t, thread.length]
  );
  const primarySectionItems = useMemo(
    () =>
      PRIMARY_SECTION_ITEMS.map((item) => ({
        ...item,
        label: t(`shell:primarySections.${item.id}`),
      })),
    [t]
  );
  const timelineViewItems = useMemo<Array<{ id: TimelineWorkspaceView; label: string }>>(
    () => [
      { id: 'feed', label: t('shell:workspace.feed') },
      { id: 'bookmarks', label: t('shell:workspace.bookmarks') },
    ],
    [t]
  );
  const settingsSectionCopy = useMemo(
    () =>
      SETTINGS_SECTION_COPY.map((section) => ({
        ...section,
        label: t(`shell:settingsSections.${section.id}.label`),
        description: t(`shell:settingsSections.${section.id}.description`),
      })),
    [t]
  );

  return {
    channelAudienceOptions,
    privateChannelListItems,
    floatingActionLabel,
    showFloatingActionButton,
    liveSessionListItems,
    gameDraftViews,
    profileEditorFields,
    profileEditorPictureSrc,
    profileEditorHasPicture,
    communityNodeStatusByBaseUrl,
    topicDiagnostics,
    effectivePeerIds,
    activeTimelinePostViews,
    bookmarkedTimelinePostViews,
    profileTimelinePostViews,
    selectedAuthorTimelinePostViews,
    threadPostViews,
    composerSourcePreview,
    topicNavItems,
    composerDraftViews,
    mentionCandidates,
    directMessageDraftViews,
    threadPanelState,
    authorDetailView,
    connectivityPanelView,
    appearancePanelView,
    discoveryPanelView,
    communityNodePanelView,
    reactionsPanelView,
    primarySectionItems,
    timelineViewItems,
    settingsSectionCopy,
    activeComposeChannel,
    activeComposeAudienceLabel,
    activeTimelineScope,
    activeJoinedChannels,
    activeLiveSessions,
    activeGameRooms,
    activeSocialConnections,
    activeSocialConnectionViews,
    activeChannelPanelState,
    activeLivePanelState,
    activeGamePanelState,
    selectedDirectMessageConversation,
    selectedDirectMessageTimeline,
    selectedDirectMessageStatus,
    selectedDirectMessagePeerLabel,
    selectedDirectMessagePeerPicture,
    selectedDirectMessagePeerAuthor,
    localDirectMessageAuthorPicture,
    directMessages,
    activePrivateChannel,
  };
}
