import {
  startTransition,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { Bell, BookPlus, Download, GitBranchPlus, PanelLeftOpen, Settings } from 'lucide-react';

import { FilterableTopicNavList } from '@/components/core/FilterableTopicNavList';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { type PrimarySection } from '@/components/shell/types';
import { StatusBadge } from '@/components/StatusBadge';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import { runtimeApi } from '@/lib/api';
import i18n from '@/i18n';
import { getResolvedLocale } from '@/i18n/format';
import {
  buildTopicLink,
  type InternalSmartReference,
} from '@/lib/internalLinks';
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
  formatCount,
  mergeKnownAuthors,
  privateComposeTarget,
  privateTimelineScope,
  syncStatusBadgeLabel,
  syncStatusBadgeTone,
  selectShellPageSlice,
} from '@/shell/selectors';
import { setRecordEntry } from '@/shell/stateUpdates';
import { useDesktopShellData } from '@/shell/useDesktopShellData';
import { useDesktopShellRouting } from '@/shell/useDesktopShellRouting';
import { useDesktopShellActions } from '@/shell/useDesktopShellActions';
import { useOsNotificationBridge } from '@/shell/useOsNotificationBridge';
import { useOsNotificationActivation } from '@/shell/useOsNotificationActivation';
import { selectUpdateAvailable, useAppUpdateStore } from '@/shell/useAppUpdateStore';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import {
  DesktopShellDetailPaneStack,
  DesktopShellMessagesWorkspace,
  DesktopShellNotificationsWorkspace,
} from '@/shell/page/DesktopShellAuxiliaryPanels';
import { DesktopShellOverlays } from '@/shell/page/DesktopShellOverlays';
import { DesktopShellPrimaryWorkspace } from '@/shell/page/DesktopShellPrimaryWorkspace';
import { DesktopShellSettingsDrawer } from '@/shell/page/DesktopShellSettingsDrawer';
import { useFocusScroll } from '@/shell/page/useFocusScroll';
import { useSharePreview } from '@/shell/page/useSharePreview';
import { useShallow } from 'zustand/react/shallow';

const CLIPBOARD_TOAST_TIMEOUT_MS = 2200;
const UPDATE_CHECK_INTERVAL_MS = 30 * 60 * 1000;

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
    syncStatus,
    selectedAuthorPubkey,
    notifications,
    notificationStatus,
    selectedLiveSessionId,
    selectedGameRoomId,
    shellChromeState,
  } = useDesktopShellStore(useShallow(selectShellPageSlice));
  const [composeDialogOpen, setComposeDialogOpen] = useState(false);
  const [channelDialogOpen, setChannelDialogOpen] = useState(false);
  const [channelSettingsDialogOpen, setChannelSettingsDialogOpen] = useState(false);
  const [leaveChannelDialogOpen, setLeaveChannelDialogOpen] = useState(false);
  const [pendingLeaveChannel, setPendingLeaveChannel] = useState<{
    topicId: string;
    channelId: string;
  } | null>(null);
  const [liveCreateDialogOpen, setLiveCreateDialogOpen] = useState(false);
  const [gameCreateDialogOpen, setGameCreateDialogOpen] = useState(false);
  const [profileAvatarPreviewUrl, setProfileAvatarPreviewUrl] = useState<string | null>(null);
  const [profileAvatarCropFile, setProfileAvatarCropFile] = useState<File | null>(null);
  const [profileAvatarCropOpen, setProfileAvatarCropOpen] = useState(false);
  const [profileAvatarInputKey, setProfileAvatarInputKey] = useState(0);
  const [clipboardToastId, setClipboardToastId] = useState(0);
  const previousPrimarySectionRef = useRef(shellChromeState.activePrimarySection);
  const previousTimelineViewRef = useRef(shellChromeState.timelineView);
  const clipboardToastTimeoutRef = useRef<number | null>(null);

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
  const updateAvailable = useAppUpdateStore(selectUpdateAvailable);
  const checkForUpdate = useAppUpdateStore((state) => state.checkForUpdate);
  const setSelectedChannelIdByTopic = useDesktopShellFieldSetter('selectedChannelIdByTopic');
  const setComposeChannelByTopic = useDesktopShellFieldSetter('composeChannelByTopic');
  const setTimelineScopeByTopic = useDesktopShellFieldSetter('timelineScopeByTopic');
  const setSelectedLiveSessionId = useDesktopShellFieldSetter('selectedLiveSessionId');
  const setSelectedGameRoomId = useDesktopShellFieldSetter('selectedGameRoomId');
  const setInviteOutput = useDesktopShellFieldSetter('inviteOutput');
  const setChannelError = useDesktopShellFieldSetter('channelError');
  const setSocialConnections = useDesktopShellFieldSetter('socialConnections');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());
  const directMessageDraftPreviewUrlRef = useRef(new Map<string, string>());
  const loadTopicsRequestRef = useRef(0);

  // Mention suggestions need the followed-users list, which is otherwise only
  // loaded in the profile section. Fetch it lazily when the composer opens.
  useEffect(() => {
    if (!composeDialogOpen) {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const following = await api.listSocialConnections('following');
        if (disposed) {
          return;
        }
        startTransition(() => {
          setSocialConnections((current) => ({ ...current, following }));
          setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, following));
        });
      } catch {
        // best effort: fall back to already-known authors for suggestions
      }
    })();
    return () => {
      disposed = true;
    };
  }, [api, composeDialogOpen, setKnownAuthorsByPubkey, setSocialConnections]);
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
  } = viewModels;
  const notificationBadgeLabel =
    notificationStatus.unread_count > 99 ? '99+' : formatCount(notificationStatus.unread_count);
  useOsNotificationBridge();
  useOsNotificationActivation(notifications, handleOpenNotification);
  const syncTopicContext = useCallback(
    async (topic: string, channelId: string | null) => {
      const nextTopics = trackedTopics.includes(topic) ? trackedTopics : [...trackedTopics, topic];
      if (!trackedTopics.includes(topic)) {
        setTrackedTopics(nextTopics);
      }
      setActiveTopic(topic);
      setSelectedChannelIdByTopic(setRecordEntry(topic, channelId));
      setTimelineScopeByTopic(setRecordEntry(topic, privateTimelineScope(channelId)));
      setComposeChannelByTopic(setRecordEntry(topic, privateComposeTarget(channelId)));
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
  const sharePreview = useSharePreview({
    api,
    importChannelAccessToken: handleImportChannelAccessToken,
    translate,
  });
  const handleOpenSharePreview = sharePreview.openPreview;
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
  useFocusScroll({
    focusKey: threadFocusKey,
    readinessKey: threadPostViews.length,
    selector: focusedObjectId ? `[data-post-object-id="${focusedObjectId}"]` : null,
  });
  const liveFocusKey =
    shellChromeState.activePrimarySection === 'live' ? selectedLiveSessionId : null;
  useFocusScroll({
    focusKey: liveFocusKey,
    readinessKey: liveSessionListItems.length,
    selector: liveFocusKey ? `[data-live-session-id="${liveFocusKey}"]` : null,
  });
  const gameFocusKey =
    shellChromeState.activePrimarySection === 'game' ? selectedGameRoomId : null;
  useFocusScroll({
    focusKey: gameFocusKey,
    readinessKey: activeGameRooms.length,
    selector: gameFocusKey ? `[data-game-room-id="${gameFocusKey}"]` : null,
  });
  useEffect(() => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
      return;
    }
    void checkForUpdate();
    const intervalId = window.setInterval(() => {
      void checkForUpdate();
    }, UPDATE_CHECK_INTERVAL_MS);
    return () => window.clearInterval(intervalId);
  }, [checkForUpdate]);
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
      {updateAvailable ? (
        <Button
          className='shell-update-button shell-icon-button'
          variant='ghost'
          size='icon'
          type='button'
          aria-label={t('shell:navigation.updateAvailable')}
          data-testid='shell-update-trigger'
          onClick={() => {
            setSettingsOpen(true, false);
            setShellChromeState((current) => ({
              ...current,
              activeSettingsSection: 'release',
            }));
            syncRoute('replace', {
              settingsOpen: true,
              settingsSection: 'release',
            });
          }}
        >
          <Download className='size-5' aria-hidden='true' />
        </Button>
      ) : null}
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
    <FilterableTopicNavList
      items={topicNavItems}
      onSelectTopic={(topic) => void handleSelectTopic(topic)}
      onSelectChannel={(topic, channelId) => {
        handleSelectPrivateChannel(topic, channelId);
      }}
      onOpenChannelSettings={(topic, channelId) => {
        setInviteOutput(null);
        setChannelError(null);
        handleSelectPrivateChannel(topic, channelId);
        setChannelSettingsDialogOpen(true);
      }}
      onLeaveChannel={(topic, channelId) => {
        setPendingLeaveChannel({ topicId: topic, channelId });
        setLeaveChannelDialogOpen(true);
      }}
      onRemoveTopic={(topic) => void handleRemoveTopic(topic)}
      onCopyTopicLink={(topic) => handleCopyInternalLink(buildTopicLink(topic))}
      onToggleTopicGossip={(topic, enabled) => void handleToggleTopicGossip(topic, enabled)}
      onToggleChannelGossip={(topic, channelId, enabled) =>
        void handleToggleChannelGossip(topic, channelId, enabled)
      }
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
      locale={locale}
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
            topicList={topicList}
            topicCount={syncStatus.subscribed_topics.length}
          />
        }
        workspace={
          <DesktopShellPrimaryWorkspace
            t={t}
            api={api}
            locale={locale}
            routeSection={routeSection}
            profileAvatarInputKey={profileAvatarInputKey}
            messagesWorkspace={messagesWorkspace}
            notificationsWorkspace={notificationsWorkspace}
            viewModels={viewModels}
            setPrimarySectionRef={setPrimarySectionRef}
            focusPrimarySection={focusPrimarySection}
            focusTimelineView={focusTimelineView}
            openCommunityNodeSettings={() => {
              setShellChromeState((current) => ({
                ...current,
                settingsOpen: true,
                activeSettingsSection: 'community-node',
              }));
              syncRoute('push', {
                settingsOpen: true,
                settingsSection: 'community-node',
              });
            }}
            loadReactionCatalogData={loadReactionCatalogData}
            refreshTimelineFeed={refreshTimelineFeed}
            refreshCurrentTopic={() => loadTopics(trackedTopics, activeTopic, selectedThread)}
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
        viewModels={viewModels}
        profileAvatarCropOpen={profileAvatarCropOpen}
        profileAvatarCropFile={profileAvatarCropFile}
        setProfileAvatarCropOpen={setProfileAvatarCropOpen}
        setProfileAvatarCropFile={setProfileAvatarCropFile}
        handleProfileAvatarFile={handleProfileAvatarFile}
        channelDialogOpen={channelDialogOpen}
        setChannelDialogOpen={setChannelDialogOpen}
        channelSettingsDialogOpen={channelSettingsDialogOpen}
        setChannelSettingsDialogOpen={setChannelSettingsDialogOpen}
        leaveChannelDialogOpen={leaveChannelDialogOpen}
        setLeaveChannelDialogOpen={(open) => {
          setLeaveChannelDialogOpen(open);
          if (!open) {
            setPendingLeaveChannel(null);
          }
        }}
        handleConfirmLeaveChannel={async () => {
          if (!pendingLeaveChannel) {
            return;
          }
          await handleLeavePrivateChannel(
            pendingLeaveChannel.topicId,
            pendingLeaveChannel.channelId
          );
          setLeaveChannelDialogOpen(false);
          setPendingLeaveChannel(null);
        }}
        sharePreviewOpen={sharePreview.open}
        handleSharePreviewOpenChange={sharePreview.handleOpenChange}
        sharePreviewToken={sharePreview.token}
        sharePreviewData={sharePreview.data}
        sharePreviewLoading={sharePreview.loading}
        sharePreviewError={sharePreview.error}
        shareImportPending={sharePreview.importPending}
        handleConfirmShareImport={sharePreview.confirmImport}
        handleCreatePrivateChannel={handleCreatePrivateChannel}
        handleJoinChannelAccess={handleJoinChannelAccess}
        handleShareChannelAccess={handleShareChannelAccess}
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
