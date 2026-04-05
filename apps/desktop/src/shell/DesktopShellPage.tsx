import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { Lock, PanelLeftOpen, Plus, Settings } from 'lucide-react';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { AuthorIdentityButton } from '@/components/core/AuthorIdentityButton';
import { ComposerDraftPreviewList } from '@/components/core/ComposerDraftPreviewList';
import { ComposerPanel } from '@/components/core/ComposerPanel';
import { ThreadPanel } from '@/components/core/ThreadPanel';
import { TimelineFeed } from '@/components/core/TimelineFeed';
import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TopicNavList } from '@/components/core/TopicNavList';
import { ProfileOverviewPanel } from '@/components/extended/ProfileOverviewPanel';
import { ProfileEditorPanel } from '@/components/extended/ProfileEditorPanel';
import { ProfileConnectionsPanel } from '@/components/extended/ProfileConnectionsPanel';
import { PrivateChannelPanel } from '@/components/extended/PrivateChannelPanel';
import { AppearancePanel } from '@/components/settings/AppearancePanel';
import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { ConnectivityPanel } from '@/components/settings/ConnectivityPanel';
import { DiscoveryPanel } from '@/components/settings/DiscoveryPanel';
import { ReactionsPanel } from '@/components/settings/ReactionsPanel';
import { ContextPane } from '@/components/shell/ContextPane';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { SettingsDrawer } from '@/components/shell/SettingsDrawer';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import { type PrimarySection } from '@/components/shell/types';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

import {
  type GameRoomStatus,
  runtimeApi,
} from '@/lib/api';
import i18n, { type SupportedLocale } from '@/i18n';
import { formatLocalizedTime, getResolvedLocale } from '@/i18n/format';
import {
  SHELL_CONTEXT_ID,
  SHELL_NAV_ID,
  SHELL_SETTINGS_ID,
  SHELL_WORKSPACE_ID,
  type DesktopShellPageProps,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  selectPrimaryImageAttachment,
  selectVideoManifestAttachment,
  selectVideoPosterAttachment,
} from '@/shell/media';
import {
  audienceLabelForTimelineScope,
  authorDisplayLabel,
  authorViewFromDirectMessageConversation,
  communityNodesToEditorValue,
  formatCount,
  localizeAudienceLabel,
  resolveProfilePictureSrc,
  seedPeersToEditorValue,
  syncStatusBadgeLabel,
  syncStatusBadgeTone,
  translateAudienceKindLabel,
  translateGameStatus,
  translateLiveStatus,
} from '@/shell/selectors';
import { useDesktopShellData } from '@/shell/useDesktopShellData';
import { useDesktopShellRouting } from '@/shell/useDesktopShellRouting';
import { useDesktopShellActions } from '@/shell/useDesktopShellActions';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';

export function DesktopShellPage({
  api = runtimeApi,
  theme,
  onThemeChange,
}: DesktopShellPageProps) {
  const { t, i18n: i18nInstance } = useTranslation([
    'common',
    'shell',
    'settings',
    'profile',
    'channels',
    'live',
    'game',
  ]);
  const locale = getResolvedLocale(i18nInstance.resolvedLanguage);
  const translate = useCallback((key: string, options?: Record<string, unknown>) => {
    return i18n.t(key, options) as string;
  }, []);
  const {
    trackedTopics,
    activeTopic,
    topicInput,
    composer,
    attachmentInputKey,
    selectedThread,
    replyTarget,
    repostTarget,
    discoveryConfig,
    discoveryEditorDirty,
    communityNodeConfig,
    communityNodeEditorDirty,
    mediaObjectUrls,
    unsupportedVideoManifests,
    syncStatus,
    localProfile,
    knownAuthorsByPubkey,
    socialConnections,
    socialConnectionsPanelState,
    ownedReactionAssets,
    bookmarkedReactionAssets,
    bookmarkedPosts,
    recentReactions,
    profileDirty,
    profileError,
    profilePanelState,
    profileSaving,
    selectedAuthorPubkey,
    selectedAuthor,
    selectedDirectMessagePeerPubkey,
    directMessages,
    directMessageComposer,
    directMessageAttachmentInputKey,
    directMessageError,
    directMessageSending,
    composerError,
    liveTitle,
    liveDescription,
    liveError,
    liveCreatePending,
    channelLabelInput,
    channelAudienceInput,
    inviteTokenInput,
    inviteOutput,
    inviteOutputLabel,
    channelError,
    channelActionPending,
    gameTitle,
    gameDescription,
    gameParticipantsInput,
    gameError,
    gameCreatePending,
    gameSavingByRoomId,
    reactionCreatePending,
    shellChromeState,
  } = useDesktopShellStore();
  const [composeDialogOpen, setComposeDialogOpen] = useState(false);
  const [channelDialogOpen, setChannelDialogOpen] = useState(false);
  const [liveCreateDialogOpen, setLiveCreateDialogOpen] = useState(false);
  const [gameCreateDialogOpen, setGameCreateDialogOpen] = useState(false);
  const [profileAvatarPreviewUrl, setProfileAvatarPreviewUrl] = useState<string | null>(null);
  const [profileAvatarInputKey, setProfileAvatarInputKey] = useState(0);
  const previousPrimarySectionRef = useRef(shellChromeState.activePrimarySection);
  const previousTimelineViewRef = useRef(shellChromeState.timelineView);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  useEffect(
    () => () => {
      if (profileAvatarPreviewUrl) {
        URL.revokeObjectURL(profileAvatarPreviewUrl);
      }
    },
    [profileAvatarPreviewUrl]
  );

  useEffect(() => {
    const previousPrimarySection = previousPrimarySectionRef.current;
    const previousTimelineView = previousTimelineViewRef.current;
    const enteredBookmarkTimeline =
      previousPrimarySection === 'timeline' &&
      shellChromeState.activePrimarySection === 'timeline' &&
      previousTimelineView !== 'bookmarks' &&
      shellChromeState.timelineView === 'bookmarks';

    if (
      (shellChromeState.activePrimarySection !== 'timeline' || enteredBookmarkTimeline) &&
      composeDialogOpen
    ) {
      setComposeDialogOpen(false);
    }
    if (shellChromeState.activePrimarySection !== 'live' && liveCreateDialogOpen) {
      setLiveCreateDialogOpen(false);
    }
    if (shellChromeState.activePrimarySection !== 'game' && gameCreateDialogOpen) {
      setGameCreateDialogOpen(false);
    }
    previousPrimarySectionRef.current = shellChromeState.activePrimarySection;
    previousTimelineViewRef.current = shellChromeState.timelineView;
  }, [
    composeDialogOpen,
    gameCreateDialogOpen,
    liveCreateDialogOpen,
    shellChromeState.activePrimarySection,
    shellChromeState.timelineView,
  ]);

  const setTopicInput = useDesktopShellFieldSetter('topicInput');
  const setPeerTicket = useDesktopShellFieldSetter('peerTicket');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setDiscoveryEditorDirty = useDesktopShellFieldSetter('discoveryEditorDirty');
  const setDiscoveryError = useDesktopShellFieldSetter('discoveryError');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setCommunityNodeEditorDirty = useDesktopShellFieldSetter('communityNodeEditorDirty');
  const setCommunityNodeError = useDesktopShellFieldSetter('communityNodeError');
  const setComposer = useDesktopShellFieldSetter('composer');
  const setDirectMessageComposer = useDesktopShellFieldSetter('directMessageComposer');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');
  const setChannelLabelInput = useDesktopShellFieldSetter('channelLabelInput');
  const setChannelAudienceInput = useDesktopShellFieldSetter('channelAudienceInput');
  const setInviteTokenInput = useDesktopShellFieldSetter('inviteTokenInput');
  const setLiveTitle = useDesktopShellFieldSetter('liveTitle');
  const setLiveDescription = useDesktopShellFieldSetter('liveDescription');
  const setGameTitle = useDesktopShellFieldSetter('gameTitle');
  const setGameDescription = useDesktopShellFieldSetter('gameDescription');
  const setGameParticipantsInput = useDesktopShellFieldSetter('gameParticipantsInput');
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());
  const directMessageDraftPreviewUrlRef = useRef(new Map<string, string>());
  const loadTopicsRequestRef = useRef(0);
  const pendingRouteUrlRef = useRef<string | null>(null);
  const didSyncRouteSectionRef = useRef(false);
  const navTriggerRef = useRef<HTMLButtonElement | null>(null);
  const settingsTriggerRef = useRef<HTMLButtonElement | null>(null);
  const primarySectionRefs = useRef<Record<PrimarySection, HTMLElement | null>>({
    timeline: null,
    live: null,
    game: null,
    messages: null,
    profile: null,
  });

  const {
    loadTopics,
    rememberDraftPreview,
    releaseDraftPreview,
    releaseAllDraftPreviews,
    rememberDirectMessageDraftPreview,
    releaseDirectMessageDraftPreview,
    releaseAllDirectMessageDraftPreviews,
    buildImageDraftItem: buildComposerImageDraftItem,
    buildVideoDraftItem: buildComposerVideoDraftItem,
  } = useDesktopShellData({
    api,
    translate,
    loadTopicsRequestRef,
    remoteObjectUrlRef,
    draftPreviewUrlRef,
    directMessageDraftPreviewUrlRef,
    mediaFetchAttemptRef,
    draftSequenceRef,
  });

  const {
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
  } = useDesktopShellRouting({
    api,
    translate,
    loadTopics,
    primarySectionRefs,
    navTriggerRef,
    settingsTriggerRef,
    pendingRouteUrlRef,
    didSyncRouteSectionRef,
  });

  const {
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
  } = useDesktopShellActions({
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
    buildImageDraftItem: buildComposerImageDraftItem,
    buildVideoDraftItem: buildComposerVideoDraftItem,
  });

  const {
    channelAudienceOptions,
    privateChannelListItems,
    floatingActionLabel,
    showFloatingActionButton,
    liveSessionListItems,
    gameDraftViews,
    profileEditorFields,
    profileEditorPictureSrc,
    profileEditorHasPicture,
    activeTimelinePostViews,
    bookmarkedTimelinePostViews,
    profileTimelinePostViews,
    selectedAuthorTimelinePostViews,
    threadPostViews,
    composerSourcePreview,
    topicNavItems,
    composerDraftViews,
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
    activeComposeAudienceLabel,
    activeTimelineScope,
    activeJoinedChannels,
    activeGameRooms,
    activeSocialConnectionViews,
    activeChannelPanelState,
    activeLivePanelState,
    activeGamePanelState,
    selectedDirectMessageTimeline,
    selectedDirectMessageStatus,
    selectedDirectMessagePeerLabel,
    selectedDirectMessagePeerPicture,
    localDirectMessageAuthorPicture,
    activePrivateChannel,
  } = useDesktopShellViewModels({
    t,
    translate,
    locale,
    theme,
    profileAvatarPreviewUrl,
  });

  const bookmarkedPostIds = useMemo(
    () => new Set(bookmarkedPosts.map((item) => item.post.object_id)),
    [bookmarkedPosts]
  );
  const profileMode = shellChromeState.profileMode;
  const profileConnectionsView = shellChromeState.profileConnectionsView;
  const navRailHeader = (
    <div className='shell-nav-status'>
      <div className='shell-status-badges'>
        <StatusBadge
          label={syncStatusBadgeLabel(syncStatus)}
          tone={syncStatusBadgeTone(syncStatus)}
        />
        <StatusBadge label={`${formatCount(syncStatus.peer_count)} ${t('settings:connectivity.metrics.peers').toLowerCase()}`} />
        <StatusBadge
          label={
            syncStatus.discovery.mode === 'seeded_dht'
              ? t('shell:navigation.seededDht')
              : t('shell:navigation.staticPeers')
          }
        />
        {syncStatus.pending_events > 0 ? (
          <StatusBadge
            label={`${formatCount(syncStatus.pending_events)} ${t('settings:connectivity.metrics.pending').toLowerCase()}`}
            tone='warning'
          />
        ) : null}
      </div>
      <Button
        ref={settingsTriggerRef}
        className='shell-settings-button shell-icon-button'
        variant='ghost'
        size='icon'
        type='button'
        aria-label={
          shellChromeState.settingsOpen
            ? t('shell:settingsDrawer.close')
            : t('shell:settingsDrawer.open')
        }
        aria-controls={SHELL_SETTINGS_ID}
        aria-expanded={shellChromeState.settingsOpen}
        data-testid='shell-settings-trigger'
        onClick={() => setSettingsOpen(!shellChromeState.settingsOpen)}
      >
        <Settings className='size-5' aria-hidden='true' />
      </Button>
    </div>
  );

  const topicList = (
    <TopicNavList
      items={topicNavItems}
      onSelectTopic={(topic) => void handleSelectTopic(topic)}
      onSelectChannel={(topic, channelId) => {
        handleSelectPrivateChannel(topic, channelId);
      }}
      onRemoveTopic={(topic) => void handleRemoveTopic(topic)}
    />
  );
  const channelAction = (
    <div className='shell-nav-channel-actions'>
      <Button
        className='shell-icon-button shell-nav-channel-action'
        variant='secondary'
        size='icon'
        type='button'
        aria-label={t('channels:title')}
        onClick={() => setChannelDialogOpen(true)}
      >
        <Lock className='size-4' aria-hidden='true' />
      </Button>
    </div>
  );

  const settingsSections = [
    {
      ...settingsSectionCopy[0],
      content: (
        <AppearancePanel
          view={appearancePanelView}
          onThemeChange={onThemeChange}
          onLocaleChange={(nextLocale: SupportedLocale) => {
            void i18nInstance.changeLanguage(nextLocale);
          }}
        />
      ),
    },
    {
      ...settingsSectionCopy[1],
      content: (
        <ConnectivityPanel
          view={connectivityPanelView}
          onPeerTicketInputChange={setPeerTicket}
          onImportPeer={() => void handleImportPeer()}
        />
      ),
    },
    {
      ...settingsSectionCopy[2],
      content: (
        <DiscoveryPanel
          view={discoveryPanelView}
          saveDisabled={discoveryConfig.env_locked || !discoveryEditorDirty}
          resetDisabled={!discoveryEditorDirty}
          onSeedPeersChange={(value) => {
            setDiscoverySeedInput(value);
            setDiscoveryEditorDirty(true);
          }}
          onSave={() => void handleSaveDiscoverySeeds()}
          onReset={() => {
            setDiscoverySeedInput(seedPeersToEditorValue(discoveryConfig));
            setDiscoveryEditorDirty(false);
            setDiscoveryError(null);
          }}
        />
      ),
    },
    {
      ...settingsSectionCopy[3],
      content: (
        <CommunityNodePanel
          view={communityNodePanelView}
          saveDisabled={!communityNodeEditorDirty}
          resetDisabled={!communityNodeEditorDirty}
          clearDisabled={communityNodeConfig.nodes.length === 0}
          onBaseUrlsChange={(value) => {
            setCommunityNodeInput(value);
            setCommunityNodeEditorDirty(true);
          }}
          onSaveNodes={() => void handleSaveCommunityNodes()}
          onReset={() => {
            setCommunityNodeInput(communityNodesToEditorValue(communityNodeConfig));
            setCommunityNodeEditorDirty(false);
            setCommunityNodeError(null);
          }}
          onClearNodes={() => void handleClearCommunityNodes()}
          onAuthenticate={(baseUrl) => void handleAuthenticateCommunityNode(baseUrl)}
          onFetchConsents={(baseUrl) => void handleFetchCommunityNodeConsents(baseUrl)}
          onAcceptConsents={(baseUrl) => void handleAcceptCommunityNodeConsents(baseUrl)}
          onRefresh={(baseUrl) => void handleRefreshCommunityNode(baseUrl)}
          onClearToken={(baseUrl) => void handleClearCommunityNodeToken(baseUrl)}
        />
      ),
    },
    {
      ...settingsSectionCopy[4],
      content: (
        <ReactionsPanel
          view={reactionsPanelView}
          creating={reactionCreatePending}
          mediaObjectUrls={mediaObjectUrls}
          onCreateAsset={(file, cropRect, searchKey) =>
            void handleCreateCustomReactionAsset(file, cropRect, searchKey)
          }
          onRemoveBookmark={(assetId) => void handleRemoveBookmarkedCustomReaction(assetId)}
        />
      ),
    },
  ];

  const profileAuthorLabel = authorDisplayLabel(
    syncStatus.local_author_pubkey,
    localProfile?.display_name,
    localProfile?.name
  );
  const messagesWorkspace = (
    <>
      <Card className='shell-workspace-card'>
        <div className='panel-header'>
          <div>
            <h3>Messages</h3>
            <small>{formatCount(directMessages.length)} conversations</small>
          </div>
          {selectedDirectMessagePeerPubkey ? (
            <Button variant='secondary' type='button' onClick={() => openDirectMessageList('replace')}>
              All
            </Button>
          ) : null}
        </div>
        {directMessageError ? <Notice tone='destructive'>{directMessageError}</Notice> : null}
        {directMessages.length === 0 ? (
          <p className='empty'>No direct messages yet.</p>
        ) : (
          <ul className='post-list'>
            {directMessages.map((conversation) => {
              const label = authorDisplayLabel(
                conversation.peer_pubkey,
                conversation.peer_display_name,
                conversation.peer_name
              );
              const knownAuthor =
                knownAuthorsByPubkey[conversation.peer_pubkey] ??
                authorViewFromDirectMessageConversation(conversation);
              const picture = resolveProfilePictureSrc(knownAuthor, mediaObjectUrls);
              const selected = conversation.peer_pubkey === selectedDirectMessagePeerPubkey;
              return (
                <li key={conversation.peer_pubkey}>
                  <article className='post-card'>
                    <div className='post-meta'>
                      <AuthorIdentityButton
                        label={label}
                        picture={picture}
                        avatarTestId={`dm-conversation-avatar-${conversation.peer_pubkey}`}
                        onClick={() =>
                          void openAuthorDetail(conversation.peer_pubkey, {
                            historyMode: 'push',
                            preserveDirectMessageContext: true,
                            directMessagePeerPubkey: selectedDirectMessagePeerPubkey,
                          })
                        }
                      />
                      <span>
                        {conversation.last_message_at
                          ? formatLocalizedTime(conversation.last_message_at, locale)
                          : t('common:fallbacks.noEvents')}
                      </span>
                    </div>
                    <div className='post-body'>
                      <strong className='post-title'>
                        Latest: {conversation.last_message_preview ?? t('common:fallbacks.none')}
                      </strong>
                    </div>
                    <div className='post-actions'>
                      <Button
                        variant={selected ? 'primary' : 'secondary'}
                        type='button'
                        onClick={() => void openDirectMessagePane(conversation.peer_pubkey)}
                      >
                        Open
                      </Button>
                    </div>
                  </article>
                </li>
              );
            })}
          </ul>
        )}
      </Card>

      {selectedDirectMessagePeerPubkey ? (
        <>
          <Card className='shell-workspace-card'>
            <div className='shell-workspace-header'>
              <div className='shell-workspace-summary'>
                <AuthorIdentityButton
                  label={selectedDirectMessagePeerLabel ?? selectedDirectMessagePeerPubkey}
                  picture={selectedDirectMessagePeerPicture}
                  avatarSize='lg'
                  avatarTestId='dm-active-header-avatar'
                  className='relationship-badge'
                  onClick={() =>
                    void openAuthorDetail(selectedDirectMessagePeerPubkey, {
                      historyMode: 'push',
                      preserveDirectMessageContext: true,
                      directMessagePeerPubkey: selectedDirectMessagePeerPubkey,
                    })
                  }
                />
                {selectedDirectMessageStatus ? (
                  <span className='relationship-badge relationship-badge-direct'>
                    {selectedDirectMessageStatus.send_enabled
                      ? `peers ${formatCount(selectedDirectMessageStatus.peer_count)}`
                      : 'send disabled'}
                  </span>
                ) : null}
              </div>
              <div className='post-actions'>
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() =>
                    void openDirectMessagePane(selectedDirectMessagePeerPubkey, {
                      historyMode: 'replace',
                    })
                  }
                >
                  {t('common:actions.refresh')}
                </Button>
                <Button
                  variant='secondary'
                  type='button'
                  disabled={selectedDirectMessageTimeline.length === 0}
                  onClick={() => void handleClearDirectMessage(selectedDirectMessagePeerPubkey)}
                >
                  {t('common:actions.clear')}
                </Button>
              </div>
            </div>
          </Card>

          <Card className='shell-workspace-card'>
            {selectedDirectMessageTimeline.length === 0 ? (
              <p className='empty'>No messages yet.</p>
            ) : (
              <ul className='post-list'>
                {selectedDirectMessageTimeline.map((message) => {
                  const image = selectPrimaryImageAttachment(message.attachments);
                  const poster = selectVideoPosterAttachment(message.attachments);
                  const video = selectVideoManifestAttachment(message.attachments);
                  const imageSrc = image ? mediaObjectUrls[image.hash] ?? null : null;
                  const posterSrc = poster ? mediaObjectUrls[poster.hash] ?? null : null;
                  const videoSrc = video ? mediaObjectUrls[video.hash] ?? null : null;
                  const videoUnsupported = Boolean(video && unsupportedVideoManifests[video.hash]);
                  const authorPubkey = message.outgoing
                    ? syncStatus.local_author_pubkey
                    : selectedDirectMessagePeerPubkey;
                  const authorLabel = message.outgoing
                    ? profileAuthorLabel
                    : selectedDirectMessagePeerLabel ?? selectedDirectMessagePeerPubkey;
                  const authorPicture = message.outgoing
                    ? localDirectMessageAuthorPicture
                    : selectedDirectMessagePeerPicture;
                  return (
                    <li key={message.message_id}>
                      <article className='post-card'>
                        <div className='post-meta'>
                          <AuthorIdentityButton
                            label={authorLabel}
                            picture={authorPicture}
                            avatarTestId={`dm-message-avatar-${message.message_id}`}
                            onClick={() =>
                              void openAuthorDetail(authorPubkey, {
                                historyMode: 'push',
                                preserveDirectMessageContext: true,
                                directMessagePeerPubkey: selectedDirectMessagePeerPubkey,
                              })
                            }
                          />
                          <span>{formatLocalizedTime(message.created_at, locale)}</span>
                          <span className='reply-chip'>
                            {message.delivered ? 'Delivered' : 'Pending'}
                          </span>
                        </div>
                        {message.text ? (
                          <div className='post-body'>
                            <strong className='post-title'>{message.text}</strong>
                          </div>
                        ) : null}
                        {image ? (
                          imageSrc ? (
                            <div className='draft-preview-frame'>
                              <img
                                className='draft-preview-image'
                                src={imageSrc}
                                alt={t('common:media.imageAlt')}
                              />
                            </div>
                          ) : (
                            <small>{t('common:media.syncingImage')}</small>
                          )
                        ) : null}
                        {video ? (
                          videoSrc && !videoUnsupported ? (
                            <video
                              className='post-card-video'
                              controls
                              playsInline
                              poster={posterSrc ?? undefined}
                              src={videoSrc}
                            />
                          ) : posterSrc ? (
                            <div className='draft-preview-frame'>
                              <img
                                className='draft-preview-image'
                                src={posterSrc}
                                alt={t('common:media.videoPosterAlt')}
                              />
                            </div>
                          ) : (
                            <small>{t('common:media.syncingPoster')}</small>
                          )
                        ) : null}
                        <div className='post-actions'>
                          <Button
                            variant='secondary'
                            type='button'
                            onClick={() =>
                              void handleDeleteDirectMessageMessage(
                                selectedDirectMessagePeerPubkey,
                                message.message_id
                              )
                            }
                          >
                            {t('common:actions.clear')}
                          </Button>
                        </div>
                      </article>
                    </li>
                  );
                })}
              </ul>
            )}
          </Card>

          <Card className='shell-workspace-card'>
            {selectedDirectMessageStatus && !selectedDirectMessageStatus.send_enabled ? (
              <Notice tone='warning'>
                Direct message send is disabled until the relationship is mutual again.
              </Notice>
            ) : null}
            <form className='composer' onSubmit={(event) => void handleSendDirectMessage(event)}>
              <Textarea
                value={directMessageComposer}
                onChange={(event) => setDirectMessageComposer(event.target.value)}
                placeholder='Write a message'
                disabled={
                  directMessageSending || selectedDirectMessageStatus?.send_enabled === false
                }
              />
              <Label className='file-field file-field-compact'>
                <span>{t('common:fallbacks.attachment')}</span>
                <Input
                  key={directMessageAttachmentInputKey}
                  aria-label={t('common:fallbacks.attachment')}
                  type='file'
                  accept='image/*,video/*'
                  disabled={
                    directMessageSending || selectedDirectMessageStatus?.send_enabled === false
                  }
                  onChange={(event) => {
                    void handleDirectMessageAttachmentSelection(event);
                  }}
                />
              </Label>
              <ComposerDraftPreviewList
                items={directMessageDraftViews}
                onRemove={handleRemoveDirectMessageDraftAttachment}
              />
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>
                  pending outbox {formatCount(selectedDirectMessageStatus?.pending_outbox_count ?? 0)}
                </span>
              </div>
              <Button
                type='submit'
                disabled={
                  directMessageSending || selectedDirectMessageStatus?.send_enabled === false
                }
              >
                {directMessageSending ? 'Sending...' : 'Send'}
              </Button>
            </form>
          </Card>
        </>
      ) : null}
    </>
  );
  const detailPaneStack = (
    <>
      {selectedThread ? (
        <ContextPane
          paneId={`${SHELL_CONTEXT_ID}-thread`}
          title={t('shell:context.thread')}
          summary={threadPanelState.summary}
          showBackdrop={!selectedAuthorPubkey}
          stackIndex={0}
          onClose={closeThreadPane}
        >
          <ThreadPanel
            state={threadPanelState}
            posts={threadPostViews}
            onOpenAuthor={(authorPubkey) =>
              void openAuthorDetail(authorPubkey, {
                fromThread: true,
                threadId: selectedThread,
              })
            }
            onOpenThread={(threadId) => void openThread(threadId)}
            onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
            onReply={beginReply}
            onRepost={(post) => void handleSimpleRepost(post)}
            onQuoteRepost={beginQuoteRepost}
            localAuthorPubkey={syncStatus.local_author_pubkey}
            mediaObjectUrls={mediaObjectUrls}
            ownedReactionAssets={ownedReactionAssets}
            bookmarkedReactionAssets={bookmarkedReactionAssets}
            recentReactions={recentReactions}
            onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
            onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
          />
        </ContextPane>
      ) : null}
      {selectedAuthorPubkey ? (
        <ContextPane
          paneId={`${SHELL_CONTEXT_ID}-author`}
          title={t('shell:context.author')}
          summary={
            selectedAuthor
              ? authorDetailView.displayLabel
              : t('common:fallbacks.selectAuthor')
          }
          showBackdrop={true}
          stackIndex={selectedThread ? 1 : 0}
          onClose={closeAuthorPane}
        >
          <div className='shell-main-stack'>
            <AuthorDetailCard
              view={authorDetailView}
              localAuthorPubkey={syncStatus.local_author_pubkey}
              onToggleRelationship={(authorPubkey, following) =>
                void handleRelationshipAction(authorPubkey, following)
              }
              onToggleMute={(authorPubkey, muted) => void handleMuteAction(authorPubkey, muted)}
              onOpenDirectMessage={(authorPubkey) => void openDirectMessagePane(authorPubkey)}
            />
            <Card className='shell-workspace-card'>
              <TimelineFeed
                posts={selectedAuthorTimelinePostViews}
                emptyCopy={t('profile:feed.noAuthorPosts')}
                onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                onOpenThread={(threadId) => void openThread(threadId)}
                onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                onReply={beginReply}
                readOnly={true}
                onOpenOriginalTopic={(topicId) => void handleOpenOriginalTopic(topicId)}
              />
            </Card>
          </div>
        </ContextPane>
      ) : null}
    </>
  );

  return (
    <>
      <ShellFrame
        skipTargetId={SHELL_WORKSPACE_ID}
        topBar={<ShellTopBar activeTopic={activeTopic} />}
        navRail={
          <ShellNavRail
            railId={SHELL_NAV_ID}
            open={shellChromeState.navOpen}
            onOpenChange={(open) => setNavOpen(open, !open)}
            headerContent={navRailHeader}
            addTopicControl={
              <Label>
                <span>{t('shell:navigation.addTopic')}</span>
                <div className='topic-input-row'>
                  <Input
                    value={topicInput}
                    onChange={(event) => setTopicInput(event.target.value)}
                    placeholder={t('shell:navigation.placeholder')}
                  />
                  <Button variant='secondary' onClick={() => void handleAddTopic()}>
                    {t('common:actions.add')}
                  </Button>
                </div>
              </Label>
            }
            channelAction={channelAction}
            channelSummary={
              activePrivateChannel
                ? `${activePrivateChannel.label} · ${translateAudienceKindLabel(activePrivateChannel.audience_kind)}`
                : t('common:audience.public')
            }
            topicList={topicList}
            topicCount={syncStatus.subscribed_topics.length}
          />
        }
        workspace={
          <div className='shell-main-stack'>
            <Card className='shell-workspace-card shell-workspace-header-card'>
              <TimelineWorkspaceHeader
                activeSection={shellChromeState.activePrimarySection}
                items={primarySectionItems}
                onSelectSection={focusPrimarySection}
              />
            </Card>

            <section
              className='shell-section'
              ref={setPrimarySectionRef(shellChromeState.activePrimarySection)}
              tabIndex={-1}
              onFocusCapture={() =>
                setShellChromeState((current) => ({
                  ...current,
                  activePrimarySection: routeSection,
                }))
              }
            >
              {shellChromeState.activePrimarySection === 'timeline' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='shell-workspace-header'>
                      <div className='shell-workspace-summary'>
                        <div className='shell-workspace-tabs' role='tablist' aria-label={t('shell:workspace.timelineViews')}>
                          {timelineViewItems.map((item) => (
                            <button
                              key={item.id}
                              className={`shell-tab${
                                shellChromeState.timelineView === item.id ? ' shell-tab-active' : ''
                              }`}
                              role='tab'
                              type='button'
                              aria-selected={shellChromeState.timelineView === item.id}
                              onClick={() => focusTimelineView(item.id)}
                            >
                              {item.label}
                            </button>
                          ))}
                        </div>
                        {shellChromeState.timelineView === 'feed' ? (
                          <>
                            <span className='relationship-badge'>
                              {t('common:audience.viewing', {
                                audience: audienceLabelForTimelineScope(
                                  activeTimelineScope,
                                  activeJoinedChannels
                                ),
                              })}
                            </span>
                            <span className='relationship-badge relationship-badge-direct'>
                              {t('common:audience.posting', {
                                audience: activeComposeAudienceLabel,
                              })}
                            </span>
                          </>
                        ) : (
                          <span className='relationship-badge'>
                            {t('shell:workspace.savedCount', {
                              count: bookmarkedTimelinePostViews.length,
                            })}
                          </span>
                        )}
                      </div>
                      <Button
                        variant='secondary'
                        type='button'
                        onClick={() => void loadTopics(trackedTopics, activeTopic, selectedThread)}
                      >
                        {t('common:actions.refresh')}
                      </Button>
                    </div>
                    {composerError ? <Notice tone='destructive'>{composerError}</Notice> : null}
                  </Card>
                  <Card className='shell-workspace-card'>
                    {shellChromeState.timelineView === 'feed' ? (
                      <TimelineFeed
                        posts={activeTimelinePostViews}
                        emptyCopy={t('shell:workspace.noPosts')}
                        onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                        onOpenThread={(threadId) => void openThread(threadId)}
                        onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                        onReply={beginReply}
                        onRepost={(post) => void handleSimpleRepost(post)}
                        onQuoteRepost={beginQuoteRepost}
                        localAuthorPubkey={syncStatus.local_author_pubkey}
                        mediaObjectUrls={mediaObjectUrls}
                        ownedReactionAssets={ownedReactionAssets}
                        bookmarkedReactionAssets={bookmarkedReactionAssets}
                        recentReactions={recentReactions}
                        onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
                        onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
                        showBookmarkAction={true}
                        bookmarkedPostIds={bookmarkedPostIds}
                        onToggleBookmark={(post) => void handleToggleBookmarkedPost(post)}
                      />
                    ) : (
                      <TimelineFeed
                        posts={bookmarkedTimelinePostViews}
                        emptyCopy={t('shell:workspace.noBookmarks')}
                        onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                        onOpenThread={(threadId) => void openThread(threadId)}
                        onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                        onReply={beginReply}
                        onRepost={(post) => void handleSimpleRepost(post)}
                        onQuoteRepost={beginQuoteRepost}
                        localAuthorPubkey={syncStatus.local_author_pubkey}
                        mediaObjectUrls={mediaObjectUrls}
                        ownedReactionAssets={ownedReactionAssets}
                        bookmarkedReactionAssets={bookmarkedReactionAssets}
                        recentReactions={recentReactions}
                        onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
                        onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
                        showBookmarkAction={true}
                        bookmarkedPostIds={bookmarkedPostIds}
                        onToggleBookmark={(post) => void handleToggleBookmarkedPost(post)}
                      />
                    )}
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'live' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='panel-header'>
                      <div>
                        <h3>{t('live:title')}</h3>
                        <small>{t('live:summary', { count: liveSessionListItems.length })}</small>
                      </div>
                    </div>
                    {activeLivePanelState.status === 'loading' ? (
                      <Notice>{t('live:loading')}</Notice>
                    ) : null}
                    {activeLivePanelState.status === 'error' &&
                    (liveError ?? activeLivePanelState.error) ? (
                      <Notice tone='destructive'>{liveError ?? activeLivePanelState.error}</Notice>
                    ) : null}
                  </Card>
                  <Card className='shell-workspace-card'>
                    {liveSessionListItems.length === 0 && activeLivePanelState.status === 'ready' ? (
                      <p className='empty-state'>{t('live:empty')}</p>
                    ) : null}
                    <ul className='post-list'>
                      {liveSessionListItems.map(({ session, isOwner, pending }) => (
                        <li key={session.session_id}>
                          <article className='post-card' aria-busy={pending}>
                            <div className='post-meta'>
                              <span>{session.title}</span>
                              <span>{translateLiveStatus(session.status)}</span>
                              <span className='reply-chip'>{localizeAudienceLabel(session.audience_label)}</span>
                            </div>
                            <div className='post-body'>
                              <strong className='post-title'>
                                {session.description || t('common:fallbacks.noDescription')}
                              </strong>
                            </div>
                            <small>{session.session_id}</small>
                            <div className='topic-diagnostic topic-diagnostic-secondary'>
                              <span>{t('common:labels.viewers')}: {formatCount(session.viewer_count)}</span>
                              <span>
                                {t('common:labels.started')}: {formatLocalizedTime(session.started_at)}
                              </span>
                            </div>
                            {session.ended_at ? (
                              <div className='topic-diagnostic topic-diagnostic-secondary'>
                                <span>
                                  {t('common:labels.ended')}: {formatLocalizedTime(session.ended_at)}
                                </span>
                              </div>
                            ) : null}
                            <div className='post-actions'>
                              {session.joined_by_me ? (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={pending}
                                  onClick={() => void handleLeaveLiveSession(session.session_id)}
                                >
                                  {t('common:actions.leave')}
                                </Button>
                              ) : (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={pending || session.status === 'Ended'}
                                  onClick={() => void handleJoinLiveSession(session.session_id)}
                                >
                                  {t('common:actions.join')}
                                </Button>
                              )}
                              {isOwner ? (
                                <Button
                                  variant='secondary'
                                  type='button'
                                  disabled={pending || session.status === 'Ended'}
                                  onClick={() => void handleEndLiveSession(session.session_id)}
                                >
                                  {t('common:actions.end')}
                                </Button>
                              ) : null}
                            </div>
                          </article>
                        </li>
                      ))}
                    </ul>
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'game' ? (
                <>
                  <Card className='shell-workspace-card'>
                    <div className='panel-header'>
                      <div>
                        <h3>{t('game:title')}</h3>
                        <small>{t('game:summary', { count: activeGameRooms.length })}</small>
                      </div>
                    </div>
                    {activeGamePanelState.status === 'loading' ? (
                      <Notice>{t('game:loading')}</Notice>
                    ) : null}
                    {activeGamePanelState.status === 'error' &&
                    (gameError ?? activeGamePanelState.error) ? (
                      <Notice tone='destructive'>{gameError ?? activeGamePanelState.error}</Notice>
                    ) : null}
                  </Card>
                  <Card className='shell-workspace-card'>
                    {activeGameRooms.length === 0 && activeGamePanelState.status === 'ready' ? (
                      <p className='empty-state'>{t('game:empty')}</p>
                    ) : null}
                    <ul className='post-list'>
                      {activeGameRooms.map((room) => {
                        const draft = gameDraftViews[room.room_id];
                        const isOwner = room.host_pubkey === syncStatus.local_author_pubkey;
                        const pending = Boolean(gameSavingByRoomId[room.room_id]);

                        return (
                          <li key={room.room_id}>
                            <article className='post-card' aria-busy={pending}>
                              <div className='post-meta'>
                                <span>{room.title}</span>
                                <span>{translateGameStatus(room.status)}</span>
                                <span className='reply-chip'>{localizeAudienceLabel(room.audience_label)}</span>
                              </div>
                              <div className='post-body'>
                                <strong className='post-title'>
                                  {room.description || t('common:fallbacks.noDescription')}
                                </strong>
                              </div>
                              <small>{room.room_id}</small>
                              <div className='topic-diagnostic topic-diagnostic-secondary'>
                                <span>{t('common:labels.phase')}: {room.phase_label ?? t('common:fallbacks.none')}</span>
                                <span>
                                  {t('common:labels.updated')}: {formatLocalizedTime(room.updated_at)}
                                </span>
                              </div>
                              <ul className='draft-attachment-list'>
                                {room.scores.map((score) => (
                                  <li
                                    key={score.participant_id}
                                    className='draft-attachment-item score-row'
                                  >
                                    <div className='draft-attachment-content'>
                                      <strong>{score.label}</strong>
                                    </div>
                                    {isOwner ? (
                                      <Input
                                        aria-label={`${room.room_id}-${score.label}-score`}
                                        value={
                                          draft?.scores[score.participant_id] ?? String(score.score)
                                        }
                                        disabled={pending}
                                        onChange={(event) =>
                                          updateGameDraft(room.room_id, (current) => ({
                                            ...current,
                                            scores: {
                                              ...current.scores,
                                              [score.participant_id]: event.target.value,
                                            },
                                          }))
                                        }
                                      />
                                    ) : (
                                      <span>{score.score}</span>
                                    )}
                                  </li>
                                ))}
                              </ul>
                              {isOwner && draft ? (
                                <div className='composer composer-compact'>
                                  <Label>
                                    <span>{t('game:fields.status')}</span>
                                    <Select
                                      aria-label={`${room.room_id}-status`}
                                      value={draft.status}
                                      disabled={pending}
                                      onChange={(event) =>
                                        updateGameDraft(room.room_id, (current) => ({
                                          ...current,
                                          status: event.target.value as GameRoomStatus,
                                        }))
                                      }
                                    >
                                      <option value='Waiting'>{t('game:statuses.Waiting')}</option>
                                      <option value='Running'>{t('game:statuses.Running')}</option>
                                      <option value='Paused'>{t('game:statuses.Paused')}</option>
                                      <option value='Ended'>{t('game:statuses.Ended')}</option>
                                    </Select>
                                  </Label>
                                  <Label>
                                    <span>{t('game:fields.phase')}</span>
                                    <Input
                                      aria-label={`${room.room_id}-phase`}
                                      value={draft.phaseLabel}
                                      disabled={pending}
                                      onChange={(event) =>
                                        updateGameDraft(room.room_id, (current) => ({
                                          ...current,
                                          phase_label: event.target.value,
                                        }))
                                      }
                                    />
                                  </Label>
                                  <Button
                                    variant='secondary'
                                    type='button'
                                    disabled={pending}
                                    onClick={() => void handleUpdateGameRoom(room.room_id)}
                                  >
                                    {t('game:actions.saveRoom')}
                                  </Button>
                                </div>
                              ) : null}
                            </article>
                          </li>
                        );
                      })}
                    </ul>
                  </Card>
                </>
              ) : null}

              {shellChromeState.activePrimarySection === 'messages' ? messagesWorkspace : null}

              {shellChromeState.activePrimarySection === 'profile' ? (
                <>
                  {profileMode === 'edit' ? (
                    <ProfileEditorPanel
                      authorLabel={profileAuthorLabel}
                      status={profilePanelState.status}
                      saving={profileSaving}
                      dirty={profileDirty}
                      error={profileError ?? profilePanelState.error}
                      fields={profileEditorFields}
                      picturePreviewSrc={profileEditorPictureSrc}
                      hasPicture={profileEditorHasPicture}
                      pictureInputKey={profileAvatarInputKey}
                      onFieldChange={handleProfileFieldChange}
                      onPictureSelect={(event) => {
                        void handleProfileAvatarSelection(event);
                      }}
                      onPictureClear={handleClearProfileAvatar}
                      onBack={openProfileOverview}
                      onSave={handleSaveProfile}
                      onReset={resetProfileDraft}
                    />
                  ) : profileMode === 'connections' ? (
                    <ProfileConnectionsPanel
                      activeView={profileConnectionsView}
                      items={activeSocialConnectionViews}
                      localAuthorPubkey={syncStatus.local_author_pubkey}
                      status={socialConnectionsPanelState.status}
                      error={socialConnectionsPanelState.error}
                      onSelectView={openProfileConnections}
                      onToggleRelationship={(authorPubkey, following) =>
                        void handleRelationshipAction(authorPubkey, following)
                      }
                      onToggleMute={(authorPubkey, muted) =>
                        void handleMuteAction(authorPubkey, muted)
                      }
                      onBack={openProfileOverview}
                    />
                  ) : (
                    <ProfileOverviewPanel
                      authorLabel={profileAuthorLabel}
                      about={localProfile?.about ?? null}
                      picture={resolveProfilePictureSrc(localProfile, mediaObjectUrls)}
                      status={profilePanelState.status}
                      error={profileError ?? profilePanelState.error}
                      postCount={profileTimelinePostViews.length}
                      followingCount={socialConnections.following.length}
                      followedCount={socialConnections.followed.length}
                      mutedCount={socialConnections.muted.length}
                      onEdit={openProfileEditor}
                      onOpenFollowing={() => openProfileConnections('following')}
                      onOpenFollowed={() => openProfileConnections('followed')}
                      onOpenMuted={() => openProfileConnections('muted')}
                    />
                  )}
                  {profileMode !== 'connections' ? (
                    <Card className='shell-workspace-card'>
                      <TimelineFeed
                        posts={profileTimelinePostViews}
                        emptyCopy={t('profile:feed.noOwnPosts')}
                        onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                        onOpenThread={(threadId) => void openThread(threadId)}
                        onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                        onReply={beginReply}
                        readOnly={true}
                        onOpenOriginalTopic={(topicId) => void handleOpenOriginalTopic(topicId)}
                      />
                    </Card>
                  ) : null}
                </>
              ) : null}
            </section>
          </div>
        }
        detailPaneStack={detailPaneStack}
        detailPaneCount={(selectedThread ? 1 : 0) + (selectedAuthorPubkey ? 1 : 0)}
        mobileFooter={
          <Button
            ref={navTriggerRef}
            variant='secondary'
            type='button'
            aria-label={
              shellChromeState.navOpen
                ? t('shell:navigation.close')
                : t('shell:navigation.open')
            }
            aria-controls={SHELL_NAV_ID}
            aria-expanded={shellChromeState.navOpen}
            data-testid='shell-nav-trigger'
            onClick={() => setNavOpen(!shellChromeState.navOpen)}
          >
            <PanelLeftOpen className='size-5' aria-hidden='true' />
            {t('shell:navigation.topicsButton')}
          </Button>
        }
      />

      <Dialog open={channelDialogOpen} onOpenChange={setChannelDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:title')}</DialogTitle>
            <DialogDescription>{activeTopic}</DialogDescription>
          </DialogHeader>
          <DialogBody>
            <PrivateChannelPanel
              status={activeChannelPanelState.status}
              error={channelError ?? activeChannelPanelState.error}
              pendingAction={channelActionPending}
              channelLabel={channelLabelInput}
              channelAudience={channelAudienceInput}
              channelAudienceOptions={channelAudienceOptions}
              inviteTokenInput={inviteTokenInput}
              inviteOutput={inviteOutput}
              inviteOutputLabel={inviteOutputLabel}
              channels={privateChannelListItems}
              selectedChannel={activePrivateChannel}
              onChannelLabelChange={setChannelLabelInput}
              onChannelAudienceChange={setChannelAudienceInput}
              onInviteTokenChange={setInviteTokenInput}
              onCreateChannel={(event) => void handleCreatePrivateChannel(event)}
              onJoin={(event) => void handleJoinChannelAccess(event)}
              onSelectChannel={(channelId) => handleSelectPrivateChannel(activeTopic, channelId)}
              onShare={() => void handleShareChannelAccess()}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={composeDialogOpen} onOpenChange={setComposeDialogOpen}>
        <DialogContent className='shell-compose-dialog'>
          <DialogHeader>
            <DialogTitle>
              {replyTarget
                ? t('common:actions.reply')
                : repostTarget
                  ? t('common:actions.quoteRepost')
                  : t('common:actions.publish')}
            </DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <ComposerPanel
              value={composer}
              onChange={(event) => setComposer(event.target.value)}
              onSubmit={handlePublish}
              attachmentInputKey={attachmentInputKey}
              onAttachmentSelection={(event) => {
                void handleAttachmentSelection(event);
              }}
              draftMediaItems={composerDraftViews}
              onRemoveDraftAttachment={handleRemoveDraftAttachment}
              composerError={composerError}
              audienceLabel={activeComposeAudienceLabel}
              sourcePreview={composerSourcePreview}
              replyTarget={
                replyTarget
                  ? {
                      content: replyTarget.content,
                      audienceLabel: replyTarget.audience_label,
                    }
                  : null
              }
              repostTarget={
                repostTarget
                  ? {
                      content: repostTarget.content,
                      authorLabel: authorDisplayLabel(
                        repostTarget.author_pubkey,
                        repostTarget.author_display_name,
                        repostTarget.author_name
                      ),
                    }
                  : null
              }
              onClearReply={clearReply}
              onClearRepost={clearRepost}
              attachmentsDisabled={Boolean(repostTarget)}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={liveCreateDialogOpen} onOpenChange={setLiveCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('live:actions.start')}</DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <form
              className='composer composer-compact'
              onSubmit={handleCreateLiveSession}
              aria-busy={liveCreatePending}
            >
              <Label>
                <span>{t('live:fields.title')}</span>
                <Input
                  value={liveTitle}
                  onChange={(event) => setLiveTitle(event.target.value)}
                  placeholder={t('live:fields.placeholders.title')}
                  disabled={liveCreatePending}
                />
              </Label>
              <Label>
                <span>{t('live:fields.description')}</span>
                <Textarea
                  value={liveDescription}
                  onChange={(event) => setLiveDescription(event.target.value)}
                  placeholder={t('live:fields.placeholders.description')}
                  disabled={liveCreatePending}
                />
              </Label>
              {liveError ? <p className='error error-inline'>{liveError}</p> : null}
              <Button type='submit' disabled={liveCreatePending}>
                {t('live:actions.start')}
              </Button>
            </form>
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={gameCreateDialogOpen} onOpenChange={setGameCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('game:actions.createRoom')}</DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <form
              className='composer composer-compact'
              onSubmit={handleCreateGameRoom}
              aria-busy={gameCreatePending}
            >
              <Label>
                <span>{t('game:fields.title')}</span>
                <Input
                  value={gameTitle}
                  onChange={(event) => setGameTitle(event.target.value)}
                  placeholder={t('game:fields.placeholders.title')}
                  disabled={gameCreatePending}
                />
              </Label>
              <Label>
                <span>{t('game:fields.description')}</span>
                <Textarea
                  value={gameDescription}
                  onChange={(event) => setGameDescription(event.target.value)}
                  placeholder={t('game:fields.placeholders.description')}
                  disabled={gameCreatePending}
                />
              </Label>
              <Label>
                <span>{t('game:fields.participants')}</span>
                <Input
                  value={gameParticipantsInput}
                  onChange={(event) => setGameParticipantsInput(event.target.value)}
                  placeholder={t('game:fields.placeholders.participants')}
                  disabled={gameCreatePending}
                />
              </Label>
              {gameError ? <p className='error error-inline'>{gameError}</p> : null}
              <Button type='submit' disabled={gameCreatePending}>
                {t('game:actions.createRoom')}
              </Button>
            </form>
          </DialogBody>
        </DialogContent>
      </Dialog>

      {showFloatingActionButton ? (
        <Button
          className='shell-fab'
          variant='primary'
          size='icon'
          type='button'
          data-testid='shell-fab'
          aria-label={floatingActionLabel}
          onClick={openFloatingActionDialog}
        >
          <Plus className='size-5' aria-hidden='true' />
        </Button>
      ) : null}

      <SettingsDrawer
        drawerId={SHELL_SETTINGS_ID}
        open={shellChromeState.settingsOpen}
        onOpenChange={(open) => setSettingsOpen(open, !open)}
        activeSection={shellChromeState.activeSettingsSection}
        onSectionChange={(section) =>
          {
            setShellChromeState((current) => ({
              ...current,
              activeSettingsSection: section,
            }));
            syncRoute('replace', {
              settingsOpen: true,
              settingsSection: section,
            });
          }
        }
        sections={settingsSections}
      />
    </>
  );
}
