import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { Bell, BookPlus, GitBranchPlus, PanelLeftOpen, Settings } from 'lucide-react';

import { TopicNavList } from '@/components/core/TopicNavList';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { type PrimarySection } from '@/components/shell/types';
import { StatusBadge } from '@/components/StatusBadge';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import { type ChannelAccessTokenPreview, runtimeApi } from '@/lib/api';
import i18n from '@/i18n';
import { formatLocalizedTime, getResolvedLocale } from '@/i18n/format';
import { buildTopicLink, type InternalSmartReference } from '@/lib/internalLinks';
import { CLIPBOARD_COPY_EVENT, copyTextToClipboard } from '@/lib/utils';
import {
  SHELL_NAV_ID,
  SHELL_SETTINGS_ID,
  SHELL_WORKSPACE_ID,
  type DesktopShellPageProps,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  authorDisplayLabel,
  formatCount,
  messageFromError,
  privateComposeTarget,
  privateTimelineScope,
  resolveProfilePictureSrc,
  syncStatusBadgeLabel,
  syncStatusBadgeTone,
  translateAudienceKindLabel,
} from '@/shell/selectors';
import { useDesktopShellData } from '@/shell/useDesktopShellData';
import { useDesktopShellRouting } from '@/shell/useDesktopShellRouting';
import { useDesktopShellActions } from '@/shell/useDesktopShellActions';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import {
  DesktopShellDetailPaneStack,
  DesktopShellMessagesWorkspace,
  DesktopShellNotificationsWorkspace,
} from '@/shell/page/DesktopShellAuxiliaryPanels';
import { DesktopShellOverlays } from '@/shell/page/DesktopShellOverlays';
import { DesktopShellPrimaryWorkspace } from '@/shell/page/DesktopShellPrimaryWorkspace';
import { DesktopShellSettingsDrawer } from '@/shell/page/DesktopShellSettingsDrawer';

const CLIPBOARD_TOAST_TIMEOUT_MS = 2200;

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
    selectedThread,
    focusedObjectId,
    mediaObjectUrls,
    syncStatus,
    localProfile,
    knownAuthorsByPubkey,
    selectedAuthorPubkey,
    notifications,
    notificationStatus,
    selectedLiveSessionId,
    selectedGameRoomId,
    shellChromeState,
  } = useDesktopShellStore();
  const [composeDialogOpen, setComposeDialogOpen] = useState(false);
  const [channelDialogOpen, setChannelDialogOpen] = useState(false);
  const [liveCreateDialogOpen, setLiveCreateDialogOpen] = useState(false);
  const [gameCreateDialogOpen, setGameCreateDialogOpen] = useState(false);
  const [profileAvatarPreviewUrl, setProfileAvatarPreviewUrl] = useState<string | null>(null);
  const [profileAvatarCropFile, setProfileAvatarCropFile] = useState<File | null>(null);
  const [profileAvatarCropOpen, setProfileAvatarCropOpen] = useState(false);
  const [profileAvatarInputKey, setProfileAvatarInputKey] = useState(0);
  const [sharePreviewOpen, setSharePreviewOpen] = useState(false);
  const [sharePreviewToken, setSharePreviewToken] = useState<string | null>(null);
  const [sharePreviewData, setSharePreviewData] = useState<ChannelAccessTokenPreview | null>(null);
  const [sharePreviewLoading, setSharePreviewLoading] = useState(false);
  const [sharePreviewError, setSharePreviewError] = useState<string | null>(null);
  const [sharePreviewShowRaw, setSharePreviewShowRaw] = useState(false);
  const [shareImportPending, setShareImportPending] = useState(false);
  const [clipboardToastId, setClipboardToastId] = useState(0);
  const previousPrimarySectionRef = useRef(shellChromeState.activePrimarySection);
  const previousTimelineViewRef = useRef(shellChromeState.timelineView);
  const clipboardToastTimeoutRef = useRef<number | null>(null);
  const lastThreadFocusKeyRef = useRef<string | null>(null);
  const lastLiveFocusKeyRef = useRef<string | null>(null);
  const lastGameFocusKeyRef = useRef<string | null>(null);

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

  useEffect(
    () => () => {
      if (clipboardToastTimeoutRef.current !== null) {
        window.clearTimeout(clipboardToastTimeoutRef.current);
      }
    },
    []
  );

  const showClipboardToast = useCallback(() => {
    setClipboardToastId((current) => current + 1);
    if (clipboardToastTimeoutRef.current !== null) {
      window.clearTimeout(clipboardToastTimeoutRef.current);
    }
    clipboardToastTimeoutRef.current = window.setTimeout(() => {
      setClipboardToastId(0);
      clipboardToastTimeoutRef.current = null;
    }, CLIPBOARD_TOAST_TIMEOUT_MS);
  }, []);

  useEffect(() => {
    const handleClipboardCopy = () => {
      showClipboardToast();
    };
    window.addEventListener(CLIPBOARD_COPY_EVENT, handleClipboardCopy as EventListener);
    return () => {
      window.removeEventListener(CLIPBOARD_COPY_EVENT, handleClipboardCopy as EventListener);
    };
  }, [showClipboardToast]);

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
  const setTrackedTopics = useDesktopShellFieldSetter('trackedTopics');
  const setActiveTopic = useDesktopShellFieldSetter('activeTopic');
  const setNotificationAutoReadError = useDesktopShellFieldSetter('notificationAutoReadError');
  const setNotificationPanelState = useDesktopShellFieldSetter('notificationPanelState');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');
  const setSelectedChannelIdByTopic = useDesktopShellFieldSetter('selectedChannelIdByTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setSelectedLiveSessionId = useDesktopShellFieldSetter('selectedLiveSessionId');
  const setSelectedGameRoomId = useDesktopShellFieldSetter('selectedGameRoomId');
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
    notifications: null,
  });

  const {
    loadTopics,
    refreshVisibleTimelineAfterPublish,
    refreshTimelineFeed,
    loadReactionCatalogData,
    loadMoreTimeline,
    loadMoreThread,
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
    toggleNotificationsSection,
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
  } = useDesktopShellActions({
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
    buildImageDraftItem: buildComposerImageDraftItem,
    buildVideoDraftItem: buildComposerVideoDraftItem,
  });

  const viewModels = useDesktopShellViewModels({
    t,
    translate,
    locale,
    theme,
    profileAvatarPreviewUrl,
  });

  const {
    liveSessionListItems,
    threadPostViews,
    topicNavItems,
    activeGameRooms,
    activePrivateChannel,
  } = viewModels;
  const notificationBadgeLabel =
    notificationStatus.unread_count > 99 ? '99+' : formatCount(notificationStatus.unread_count);
  const notificationItems = useMemo(
    () =>
      notifications.map((notification) => {
        const knownAuthor = knownAuthorsByPubkey[notification.actor_pubkey] ?? null;
        const actorLabel = authorDisplayLabel(
          notification.actor_pubkey,
          notification.actor_display_name,
          notification.actor_name
        );
        const actorPicture = knownAuthor
          ? resolveProfilePictureSrc(knownAuthor, mediaObjectUrls)
          : notification.actor_picture_asset
            ? mediaObjectUrls[notification.actor_picture_asset.hash] ?? notification.actor_picture ?? null
            : notification.actor_picture ?? null;
        const contextLabel =
          notification.kind === 'direct_message'
            ? t('shell:notifications.context.directMessage')
            : notification.topic_id && notification.channel_id
              ? t('shell:notifications.context.topicChannel', {
                  channel: notification.channel_id,
                  topic: notification.topic_id,
                })
              : notification.topic_id
                ? t('shell:notifications.context.topic', {
                    topic: notification.topic_id,
                  })
                : t('shell:notifications.context.authorActivity');
        const previewText =
          notification.preview_text ??
          (notification.kind === 'followed'
            ? t('shell:notifications.preview.followed')
            : notification.kind === 'direct_message'
              ? t('shell:notifications.preview.noMessage')
              : t('shell:notifications.preview.noContent'));

        return {
          ...notification,
          actorLabel,
          actorPicture,
          contextLabel,
          kindLabel: t(`shell:notifications.kinds.${notification.kind}`),
          previewText,
          receivedLabel: formatLocalizedTime(notification.received_at, locale),
          unread: !notification.read_at,
        };
      }),
    [knownAuthorsByPubkey, locale, mediaObjectUrls, notifications, t]
  );
  const syncTopicContext = useCallback(
    async (topic: string, channelId: string | null) => {
      const nextTopics = trackedTopics.includes(topic) ? trackedTopics : [...trackedTopics, topic];
      if (!trackedTopics.includes(topic)) {
        setTrackedTopics(nextTopics);
      }
      setActiveTopic(topic);
      setSelectedChannelIdByTopic((current) => ({
        ...current,
        [topic]: channelId,
      }));
      setTimelineScopeByTopic((current) => ({
        ...current,
        [topic]: privateTimelineScope(channelId),
      }));
      setComposeChannelByTopic((current) => ({
        ...current,
        [topic]: privateComposeTarget(channelId),
      }));
      await loadTopics(nextTopics, topic, null);
    },
    [
      loadTopics,
      setActiveTopic,
      setComposeChannelByTopic,
      setSelectedChannelIdByTopic,
      setTimelineScopeByTopic,
      setTrackedTopics,
      trackedTopics,
    ]
  );
  const handleCopyInternalLink = useCallback((link: string) => {
    void copyTextToClipboard(link);
  }, []);
  const handleOpenSharePreview = useCallback(
    async (token: string) => {
      setSharePreviewOpen(true);
      setSharePreviewToken(token);
      setSharePreviewData(null);
      setSharePreviewError(null);
      setSharePreviewShowRaw(false);
      setSharePreviewLoading(true);
      try {
        const preview = await api.previewChannelAccessToken(token);
        setSharePreviewData(preview);
      } catch (error) {
        setSharePreviewError(
          messageFromError(error, translate('channels:errors.failedPreviewToken'))
        );
      } finally {
        setSharePreviewLoading(false);
      }
    },
    [api, translate]
  );
  const handleConfirmShareImport = useCallback(async () => {
    if (!sharePreviewToken) {
      return;
    }
    setShareImportPending(true);
    setSharePreviewError(null);
    try {
      await handleImportChannelAccessToken(sharePreviewToken);
      setSharePreviewOpen(false);
    } catch (error) {
      setSharePreviewError(messageFromError(error, translate('channels:errors.failedJoinChannel')));
    } finally {
      setShareImportPending(false);
    }
  }, [handleImportChannelAccessToken, sharePreviewToken, translate]);
  const handleActivateReference = useCallback(
    async (reference: InternalSmartReference) => {
      if (reference.kind === 'share_token') {
        await handleOpenSharePreview(reference.token);
        return;
      }
      if (reference.kind === 'topic') {
        await syncTopicContext(reference.topic, null);
        setSelectedLiveSessionId(null);
        setSelectedGameRoomId(null);
        setShellChromeState((current) => ({
          ...current,
          activePrimarySection: 'timeline',
          timelineView: 'feed',
          navOpen: false,
        }));
        syncRoute('push', {
          activeTopic: reference.topic,
          composeTarget: PUBLIC_CHANNEL_REF,
          focusedObjectId: null,
          primarySection: 'timeline',
          selectedAuthorPubkey: null,
          selectedDirectMessagePeerPubkey: null,
          selectedGameRoomId: null,
          selectedLiveSessionId: null,
          selectedThread: null,
          timelineScope: PUBLIC_TIMELINE_SCOPE,
          timelineView: 'feed',
        });
        return;
      }
      if (reference.kind === 'post') {
        if (!trackedTopics.includes(reference.topic)) {
          setTrackedTopics([...trackedTopics, reference.topic]);
        }
        await openThread(reference.threadId, {
          focusObjectId: reference.focusObjectId ?? reference.threadId,
          topic: reference.topic,
        });
        return;
      }
      if (reference.kind === 'live') {
        await syncTopicContext(reference.topic, reference.channelId);
        setSelectedLiveSessionId(reference.sessionId);
        setSelectedGameRoomId(null);
        setShellChromeState((current) => ({
          ...current,
          activePrimarySection: 'live',
          navOpen: false,
        }));
        syncRoute('push', {
          activeTopic: reference.topic,
          composeTarget: privateComposeTarget(reference.channelId),
          focusedObjectId: null,
          primarySection: 'live',
          selectedAuthorPubkey: null,
          selectedDirectMessagePeerPubkey: null,
          selectedGameRoomId: null,
          selectedLiveSessionId: reference.sessionId,
          selectedThread: null,
          timelineScope: privateTimelineScope(reference.channelId),
        });
        return;
      }
      await syncTopicContext(reference.topic, reference.channelId);
      setSelectedGameRoomId(reference.roomId);
      setSelectedLiveSessionId(null);
      setShellChromeState((current) => ({
        ...current,
        activePrimarySection: 'game',
        navOpen: false,
      }));
      syncRoute('push', {
        activeTopic: reference.topic,
        composeTarget: privateComposeTarget(reference.channelId),
        focusedObjectId: null,
        primarySection: 'game',
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedGameRoomId: reference.roomId,
        selectedLiveSessionId: null,
        selectedThread: null,
        timelineScope: privateTimelineScope(reference.channelId),
      });
    },
    [
      handleOpenSharePreview,
      openThread,
      setSelectedGameRoomId,
      setSelectedLiveSessionId,
      setShellChromeState,
      setTrackedTopics,
      syncRoute,
      syncTopicContext,
      trackedTopics,
    ]
  );
  const threadFocusKey = selectedThread && focusedObjectId
    ? `${selectedThread}:${focusedObjectId}`
    : null;
  useEffect(() => {
    if (!threadFocusKey || lastThreadFocusKeyRef.current === threadFocusKey) {
      return;
    }
    const frameId = window.requestAnimationFrame(() => {
      const selector = `[data-post-object-id="${focusedObjectId}"]`;
      const target = document.querySelector(selector);
      if (target instanceof HTMLElement) {
        if (typeof target.scrollIntoView === 'function') {
          target.scrollIntoView({ block: 'center' });
        }
        target.focus({ preventScroll: true });
        lastThreadFocusKeyRef.current = threadFocusKey;
      }
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [focusedObjectId, threadFocusKey, threadPostViews.length]);
  const liveFocusKey =
    shellChromeState.activePrimarySection === 'live' ? selectedLiveSessionId : null;
  useEffect(() => {
    if (!liveFocusKey || lastLiveFocusKeyRef.current === liveFocusKey) {
      return;
    }
    const frameId = window.requestAnimationFrame(() => {
      const selector = `[data-live-session-id="${liveFocusKey}"]`;
      const target = document.querySelector(selector);
      if (target instanceof HTMLElement) {
        if (typeof target.scrollIntoView === 'function') {
          target.scrollIntoView({ block: 'center' });
        }
        target.focus({ preventScroll: true });
        lastLiveFocusKeyRef.current = liveFocusKey;
      }
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [liveFocusKey, liveSessionListItems.length]);
  const gameFocusKey =
    shellChromeState.activePrimarySection === 'game' ? selectedGameRoomId : null;
  useEffect(() => {
    if (!gameFocusKey || lastGameFocusKeyRef.current === gameFocusKey) {
      return;
    }
    const frameId = window.requestAnimationFrame(() => {
      const selector = `[data-game-room-id="${gameFocusKey}"]`;
      const target = document.querySelector(selector);
      if (target instanceof HTMLElement) {
        if (typeof target.scrollIntoView === 'function') {
          target.scrollIntoView({ block: 'center' });
        }
        target.focus({ preventScroll: true });
        lastGameFocusKeyRef.current = gameFocusKey;
      }
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [activeGameRooms.length, gameFocusKey]);
  const notificationAction = (
    <Button
      className='shell-notification-button'
      variant={shellChromeState.activePrimarySection === 'notifications' ? 'primary' : 'secondary'}
      type='button'
      aria-current={shellChromeState.activePrimarySection === 'notifications' ? 'page' : undefined}
      onClick={() => {
        if (shellChromeState.activePrimarySection !== 'notifications') {
          setNotificationAutoReadError(null);
          setNotificationPanelState({
            status: 'loading',
            error: null,
          });
        }
        toggleNotificationsSection();
      }}
    >
      <Bell className='size-4' aria-hidden='true' />
      <span>{t('shell:navigation.notificationsButton')}</span>
      <Badge
        className='shell-notification-button-badge'
        tone={notificationStatus.unread_count > 0 ? 'accent' : 'neutral'}
      >
        {notificationBadgeLabel}
      </Badge>
    </Button>
  );
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
      onCopyTopicLink={(topic) => handleCopyInternalLink(buildTopicLink(topic))}
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
        <GitBranchPlus className='size-4' aria-hidden='true' />
      </Button>
    </div>
  );

  const profileAuthorLabel = authorDisplayLabel(
    syncStatus.local_author_pubkey,
    localProfile?.display_name,
    localProfile?.name
  );
  const messagesWorkspace = (
    <DesktopShellMessagesWorkspace
      t={t}
      locale={locale}
      viewModels={viewModels}
      openDirectMessageList={openDirectMessageList}
      openDirectMessagePane={openDirectMessagePane}
      openAuthorDetail={openAuthorDetail}
      handleClearDirectMessage={handleClearDirectMessage}
      handleDeleteDirectMessageMessage={handleDeleteDirectMessageMessage}
      handleDirectMessageAttachmentSelection={handleDirectMessageAttachmentSelection}
      handleRemoveDirectMessageDraftAttachment={handleRemoveDirectMessageDraftAttachment}
      handleSendDirectMessage={handleSendDirectMessage}
    />
  );
  const notificationsWorkspace = (
    <DesktopShellNotificationsWorkspace
      t={t}
      notificationItems={notificationItems}
      onRefresh={() => {
        setNotificationAutoReadError(null);
        setNotificationPanelState({
          status: 'loading',
          error: null,
        });
        void loadTopics(trackedTopics, activeTopic, null).catch(() => undefined);
      }}
      handleOpenNotification={handleOpenNotification}
    />
  );
  const detailPaneStack = (
    <DesktopShellDetailPaneStack
      t={t}
      activeTopic={activeTopic}
      viewModels={viewModels}
      closeAuthorPane={closeAuthorPane}
      closeThreadPane={closeThreadPane}
      loadMoreThread={loadMoreThread}
      loadReactionCatalogData={loadReactionCatalogData}
      openAuthorDetail={openAuthorDetail}
      openDirectMessagePane={openDirectMessagePane}
      openThread={openThread}
      beginReply={beginReply}
      handleSimpleRepost={handleSimpleRepost}
      beginQuoteRepost={beginQuoteRepost}
      handleRetryLocalPost={handleRetryLocalPost}
      handleRestoreLocalPost={handleRestoreLocalPost}
      handleToggleReaction={handleToggleReaction}
      handleBookmarkCustomReaction={handleBookmarkCustomReaction}
      handleActivateReference={handleActivateReference}
      handleCopyPostLink={handleCopyInternalLink}
      handleRelationshipAction={handleRelationshipAction}
      handleMuteAction={handleMuteAction}
      handleOpenOriginalTopic={handleOpenOriginalTopic}
    />
  );

  return (
    <>
      <ShellFrame
        skipTargetId={SHELL_WORKSPACE_ID}
        navRail={
          <ShellNavRail
            railId={SHELL_NAV_ID}
            open={shellChromeState.navOpen}
            onOpenChange={(open) => setNavOpen(open, !open)}
            notificationAction={notificationAction}
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
                  <Button
                    variant='secondary'
                    size='icon'
                    type='button'
                    aria-label={t('common:actions.add')}
                    onClick={() => void handleAddTopic()}
                  >
                    <BookPlus className='size-4' aria-hidden='true' />
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
          <DesktopShellPrimaryWorkspace
            t={t}
            locale={locale}
            routeSection={routeSection}
            profileAuthorLabel={profileAuthorLabel}
            profileAvatarInputKey={profileAvatarInputKey}
            messagesWorkspace={messagesWorkspace}
            notificationsWorkspace={notificationsWorkspace}
            viewModels={viewModels}
            setPrimarySectionRef={setPrimarySectionRef}
            focusPrimarySection={focusPrimarySection}
            focusTimelineView={focusTimelineView}
            loadReactionCatalogData={loadReactionCatalogData}
            refreshTimelineFeed={refreshTimelineFeed}
            loadMoreTimeline={loadMoreTimeline}
            openAuthorDetail={openAuthorDetail}
            openThread={openThread}
            beginReply={beginReply}
            handleSimpleRepost={handleSimpleRepost}
            beginQuoteRepost={beginQuoteRepost}
            handleRetryLocalPost={handleRetryLocalPost}
            handleRestoreLocalPost={handleRestoreLocalPost}
            handleToggleReaction={handleToggleReaction}
            handleBookmarkCustomReaction={handleBookmarkCustomReaction}
            handleToggleBookmarkedPost={handleToggleBookmarkedPost}
            handleActivateReference={handleActivateReference}
            handleCopyInternalLink={handleCopyInternalLink}
            handleJoinLiveSession={handleJoinLiveSession}
            handleLeaveLiveSession={handleLeaveLiveSession}
            handleEndLiveSession={handleEndLiveSession}
            updateGameDraft={updateGameDraft}
            handleUpdateGameRoom={handleUpdateGameRoom}
            openProfileOverview={openProfileOverview}
            openProfileEditor={openProfileEditor}
            openProfileConnections={openProfileConnections}
            handleProfileFieldChange={handleProfileFieldChange}
            onProfilePictureSelect={(file) => {
              setProfileAvatarCropFile(file);
              setProfileAvatarCropOpen(true);
            }}
            handleClearProfileAvatar={handleClearProfileAvatar}
            handleSaveProfile={handleSaveProfile}
            resetProfileDraft={resetProfileDraft}
            handleRelationshipAction={handleRelationshipAction}
            handleMuteAction={handleMuteAction}
            handleOpenOriginalTopic={handleOpenOriginalTopic}
          />
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

      <DesktopShellOverlays
        t={t}
        activeTopic={activeTopic}
        viewModels={viewModels}
        profileAvatarCropOpen={profileAvatarCropOpen}
        profileAvatarCropFile={profileAvatarCropFile}
        setProfileAvatarCropOpen={setProfileAvatarCropOpen}
        setProfileAvatarCropFile={setProfileAvatarCropFile}
        handleProfileAvatarFile={handleProfileAvatarFile}
        channelDialogOpen={channelDialogOpen}
        setChannelDialogOpen={setChannelDialogOpen}
        sharePreviewOpen={sharePreviewOpen}
        setSharePreviewOpen={setSharePreviewOpen}
        sharePreviewToken={sharePreviewToken}
        setSharePreviewToken={setSharePreviewToken}
        sharePreviewData={sharePreviewData}
        setSharePreviewData={setSharePreviewData}
        sharePreviewLoading={sharePreviewLoading}
        sharePreviewError={sharePreviewError}
        setSharePreviewError={setSharePreviewError}
        sharePreviewShowRaw={sharePreviewShowRaw}
        setSharePreviewShowRaw={setSharePreviewShowRaw}
        shareImportPending={shareImportPending}
        handleConfirmShareImport={handleConfirmShareImport}
        handleCreatePrivateChannel={handleCreatePrivateChannel}
        handleJoinChannelAccess={handleJoinChannelAccess}
        handleSelectPrivateChannel={handleSelectPrivateChannel}
        handleShareChannelAccess={handleShareChannelAccess}
        handleActivateReference={handleActivateReference}
        handleCopyInternalLink={handleCopyInternalLink}
        composeDialogOpen={composeDialogOpen}
        setComposeDialogOpen={setComposeDialogOpen}
        handlePublish={handlePublish}
        handleAttachmentSelection={handleAttachmentSelection}
        handleRemoveDraftAttachment={handleRemoveDraftAttachment}
        clearReply={clearReply}
        clearRepost={clearRepost}
        liveCreateDialogOpen={liveCreateDialogOpen}
        setLiveCreateDialogOpen={setLiveCreateDialogOpen}
        handleCreateLiveSession={handleCreateLiveSession}
        gameCreateDialogOpen={gameCreateDialogOpen}
        setGameCreateDialogOpen={setGameCreateDialogOpen}
        handleCreateGameRoom={handleCreateGameRoom}
        openFloatingActionDialog={openFloatingActionDialog}
        clipboardToastId={clipboardToastId}
      />

      <DesktopShellSettingsDrawer
        drawerId={SHELL_SETTINGS_ID}
        onThemeChange={onThemeChange}
        onLocaleChange={(nextLocale) => {
          void i18nInstance.changeLanguage(nextLocale);
        }}
        syncRoute={syncRoute}
        setSettingsOpen={setSettingsOpen}
        viewModels={viewModels}
        handleImportPeer={handleImportPeer}
        handleSaveDiscoverySeeds={handleSaveDiscoverySeeds}
        handleSaveCommunityNodes={handleSaveCommunityNodes}
        handleClearCommunityNodes={handleClearCommunityNodes}
        handleAuthenticateCommunityNode={handleAuthenticateCommunityNode}
        handleFetchCommunityNodeConsents={handleFetchCommunityNodeConsents}
        handleAcceptCommunityNodeConsents={handleAcceptCommunityNodeConsents}
        handleRefreshCommunityNode={handleRefreshCommunityNode}
        handleClearCommunityNodeToken={handleClearCommunityNodeToken}
        handleCreateCustomReactionAsset={handleCreateCustomReactionAsset}
        handleRemoveBookmarkedCustomReaction={handleRemoveBookmarkedCustomReaction}
      />
    </>
  );
}
