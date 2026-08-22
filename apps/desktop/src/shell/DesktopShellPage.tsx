import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';

import {
  CommunityIndexingRequestDialog,
  type CommunityIndexingTarget,
} from '@/components/core/CommunityIndexingRequestDialog';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { type PrimarySection, type SettingsSection } from '@/components/shell/types';

import { runtimeApi } from '@/lib/api';
import { eligibleCommunityIndexNodes } from '@/lib/api/communityIndex';
import i18n from '@/i18n';
import { getResolvedLocale } from '@/i18n/format';
import {
  buildTopicLink,
  type InternalSmartReference,
} from '@/lib/internalLinks';
import { CLIPBOARD_COPY_EVENT, copyTextToClipboard } from '@/lib/utils';
import {
  SHELL_WORKSPACE_ID,
  type DesktopShellPageProps,
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  privateComposeTarget,
  privateTimelineScope,
} from '@/shell/presentation';
import { selectShellPageSlice } from '@/shell/storeSelectors';
import { setRecordEntry } from '@/shell/stateUpdates';
import { useDesktopShellData } from '@/shell/useDesktopShellData';
import { useDesktopShellRouting } from '@/shell/useDesktopShellRouting';
import { useDesktopShellActions } from '@/shell/useDesktopShellActions';
import { useOsNotificationBridge } from '@/shell/useOsNotificationBridge';
import { useOsNotificationActivation } from '@/shell/useOsNotificationActivation';
import { selectUpdateAvailable, useAppUpdateStore } from '@/shell/useAppUpdateStore';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import {
  DesktopShellDetailSurfaceStack,
  DesktopShellMessagesSurface,
  DesktopShellNotificationsSurface,
} from '@/shell/page/DesktopShellAuxiliaryPanels';
import { DesktopShellOverlays } from '@/shell/page/DesktopShellOverlays';
import { DesktopShellColumnWorkspace } from '@/shell/page/DesktopShellColumnWorkspace';
import { DesktopShellControlCenter } from '@/shell/page/DesktopShellControlCenter';
import { DesktopShellPrimarySurface } from '@/shell/page/DesktopShellPrimaryWorkspace';
import { DesktopShellSettingsDrawer } from '@/shell/page/DesktopShellSettingsDrawer';
import { useFocusScroll } from '@/shell/page/useFocusScroll';
import { useSharePreview } from '@/shell/page/useSharePreview';
import { useShellDialogs } from '@/shell/page/useShellDialogs';
import { useDesktopShellColumnSynchronization } from '@/shell/page/useDesktopShellColumnSynchronization';
import { useShallow } from 'zustand/react/shallow';
import { type ColumnKind, type ColumnState } from '@/shell/slices/workspace';

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
    selectedChannelIdByTopic,
    notifications,
    selectedLiveSessionId,
    selectedGameRoomId,
    developerModeEnabled,
    shellChromeState,
    communityNodeConfig,
    communityNodeStatuses,
    communityNodeManifests,
  } = useDesktopShellStore(useShallow(selectShellPageSlice));
  const [profileAvatarPreviewUrl, setProfileAvatarPreviewUrl] = useState<string | null>(null);
  const [clipboardToastId, setClipboardToastId] = useState(0);
  const [indexingTarget, setIndexingTarget] = useState<CommunityIndexingTarget | null>(null);
  const clipboardToastTimeoutRef = useRef<number | null>(null);
  const dialogs = useShellDialogs({
    activePrimarySection: shellChromeState.activePrimarySection,
    api,
    timelineView: shellChromeState.timelineView,
  });

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
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());
  const directMessageDraftPreviewUrlRef = useRef(new Map<string, string>());
  const loadTopicsRequestRef = useRef(new Map<string, number>());

  const pendingRouteUrlRef = useRef<string | null>(null);
  const didSyncRouteSectionRef = useRef(false);
  const controlCenterTriggerRef = useRef<HTMLButtonElement | null>(null);
  const primarySectionRefs = useRef<Record<PrimarySection, HTMLElement | null>>({
    timeline: null,
    explore: null,
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
    navTriggerRef: controlCenterTriggerRef,
    settingsTriggerRef: controlCenterTriggerRef,
    pendingRouteUrlRef,
    didSyncRouteSectionRef,
  });

  const shellActions = useDesktopShellActions({
    api,
    translate,
    loadTopics,
    refreshVisibleTimelineAfterPublish,
    syncRoute,
    openDirectMessagePane,
    openAuthorDetail,
    openThread,
    setComposeDialogOpen: dialogs.setComposeDialogOpen,
    setLiveCreateDialogOpen: dialogs.setLiveCreateDialogOpen,
    setGameCreateDialogOpen: dialogs.setGameCreateDialogOpen,
    setProfileAvatarPreviewUrl,
    setProfileAvatarInputKey: dialogs.setProfileAvatarInputKey,
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
    topicNavItems,
    activeGameRooms,
  } = viewModels;
  useDesktopShellColumnSynchronization(activeGameRooms);
  useOsNotificationBridge();
  useOsNotificationActivation(notifications, shellActions.handleOpenNotification);
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
    importChannelAccessToken: shellActions.handleImportChannelAccessToken,
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
      if ((reference.kind === 'live' || reference.kind === 'game') && !developerModeEnabled) {
        // WIP機能が隠れている間は live/game リンクを topic timeline へ落とす。
        await syncTopicContext(reference.topic, reference.channelId);
        setSelectedLiveSessionId(null);
        setSelectedGameRoomId(null);
        setShellChromeState((current) => ({
          ...current,
          activePrimarySection: 'timeline',
          navOpen: false,
        }));
        syncRoute('push', {
          activeTopic: reference.topic,
          composeTarget: privateComposeTarget(reference.channelId),
          focusedObjectId: null,
          primarySection: 'timeline',
          selectedAuthorPubkey: null,
          selectedDirectMessagePeerPubkey: null,
          selectedGameRoomId: null,
          selectedLiveSessionId: null,
          selectedThread: null,
          timelineScope: privateTimelineScope(reference.channelId),
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
      developerModeEnabled,
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
  const eligibleIndexingNodes = useMemo(
    () =>
      eligibleCommunityIndexNodes(
        communityNodeConfig,
        communityNodeStatuses,
        communityNodeManifests
      ),
    [communityNodeConfig, communityNodeManifests, communityNodeStatuses]
  );
  const handleOpenSettingsSection = useCallback((section: SettingsSection) => {
    if (section === 'community-node') setIndexingTarget(null);
    setSettingsOpen(true, false);
    setShellChromeState((current) => ({
      ...current,
      settingsOpen: true,
      activeSettingsSection: section,
    }));
    syncRoute('push', {
      settingsOpen: true,
      settingsSection: section,
    });
  }, [setSettingsOpen, setShellChromeState, syncRoute]);
  const handleOpenCommunityNodeSettings = useCallback(
    () => handleOpenSettingsSection('community-node'),
    [handleOpenSettingsSection]
  );
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
  const renderMessagesSurface = (
    surfaceKind: 'messages' | 'conversation',
    peerPubkey?: string
  ) => (
    <DesktopShellMessagesSurface
      t={t}
      locale={locale}
      viewModels={viewModels}
      openDirectMessageList={openDirectMessageList}
      openDirectMessagePane={openDirectMessagePane}
      openAuthorDetail={openAuthorDetail}
      handleClearDirectMessage={shellActions.handleClearDirectMessage}
      handleDeleteDirectMessageMessage={shellActions.handleDeleteDirectMessageMessage}
      handleDirectMessageAttachmentSelection={shellActions.handleDirectMessageAttachmentSelection}
      handleRemoveDirectMessageDraftAttachment={shellActions.handleRemoveDirectMessageDraftAttachment}
      handleSendDirectMessage={shellActions.handleSendDirectMessage}
      surfaceKind={surfaceKind}
      peerPubkey={peerPubkey}
      showComposer={false}
    />
  );
  const notificationsSurface = (
    <DesktopShellNotificationsSurface
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
      handleOpenNotification={shellActions.handleOpenNotification}
    />
  );
  const renderDetailSurface = (
    surfaceKind: 'thread' | 'profile',
    entityId?: string,
    topicId?: string
  ) => (
    <DesktopShellDetailSurfaceStack
      api={api}
      t={t}
      viewModels={viewModels}
      closeAuthorPane={closeAuthorPane}
      closeThreadPane={closeThreadPane}
      loadMoreThread={loadMoreThread}
      loadReactionCatalogData={loadReactionCatalogData}
      openAuthorDetail={openAuthorDetail}
      openDirectMessagePane={openDirectMessagePane}
      openThread={openThread}
      beginReply={shellActions.beginColumnReply}
      handleSimpleRepost={shellActions.handleSimpleRepost}
      beginQuoteRepost={shellActions.beginQuoteRepost}
      handleRetryLocalPost={shellActions.handleRetryLocalPost}
      handleRestoreLocalPost={shellActions.handleRestoreLocalPost}
      handleToggleReaction={shellActions.handleToggleReaction}
      handleBookmarkCustomReaction={shellActions.handleBookmarkCustomReaction}
      handleActivateReference={handleActivateReference}
      handleCopyPostLink={handleCopyInternalLink}
      handleRelationshipAction={shellActions.handleRelationshipAction}
      handleMuteAction={shellActions.handleMuteAction}
      handleOpenOriginalTopic={shellActions.handleOpenOriginalTopic}
      openCommunityNodeSettings={handleOpenCommunityNodeSettings}
      surfaceKind={surfaceKind}
      entityId={entityId}
      topicId={topicId}
    />
  );
  const renderPrimarySurface = (surfaceSection: PrimarySection, column?: ColumnState) => (
    <DesktopShellPrimarySurface
      t={t}
      api={api}
      metaverseActions={shellActions.metaverseActions}
      locale={locale}
      routeSection={routeSection}
      surfaceSection={surfaceSection}
      surfaceColumnKind={column?.kind}
      surfaceEntityId={column?.entityId}
      surfaceScope={column?.scope}
      profileAvatarInputKey={dialogs.profileAvatarInputKey}
      messagesWorkspace={null}
      notificationsWorkspace={null}
      viewModels={viewModels}
      setPrimarySectionRef={setPrimarySectionRef}
      focusTimelineView={focusTimelineView}
      openCommunityNodeSettings={handleOpenCommunityNodeSettings}
      loadReactionCatalogData={loadReactionCatalogData}
      refreshTimelineFeed={refreshTimelineFeed}
      loadMoreTimeline={loadMoreTimeline}
      openAuthorDetail={openAuthorDetail}
      openThread={openThread}
      beginReply={shellActions.beginColumnReply}
      handleSimpleRepost={shellActions.handleSimpleRepost}
      beginQuoteRepost={shellActions.beginQuoteRepost}
      handleRetryLocalPost={shellActions.handleRetryLocalPost}
      handleRestoreLocalPost={shellActions.handleRestoreLocalPost}
      handleToggleReaction={shellActions.handleToggleReaction}
      handleBookmarkCustomReaction={shellActions.handleBookmarkCustomReaction}
      handleToggleBookmarkedPost={shellActions.handleToggleBookmarkedPost}
      handleActivateReference={handleActivateReference}
      handleCopyInternalLink={handleCopyInternalLink}
      handleJoinLiveSession={shellActions.handleJoinLiveSession}
      handleLeaveLiveSession={shellActions.handleLeaveLiveSession}
      handleEndLiveSession={shellActions.handleEndLiveSession}
      handleCreateGameRoom={shellActions.handleCreateGameRoom}
      updateGameDraft={shellActions.updateGameDraft}
      handleUpdateGameRoom={shellActions.handleUpdateGameRoom}
      openProfileOverview={openProfileOverview}
      openProfileEditor={openProfileEditor}
      openProfileConnections={openProfileConnections}
      handleProfileFieldChange={shellActions.handleProfileFieldChange}
      onProfilePictureSelect={(file) => {
        dialogs.setProfileAvatarCropFile(file);
        dialogs.setProfileAvatarCropOpen(true);
      }}
      handleClearProfileAvatar={shellActions.handleClearProfileAvatar}
      handleSaveProfile={shellActions.handleSaveProfile}
      resetProfileDraft={shellActions.resetProfileDraft}
      handleRelationshipAction={shellActions.handleRelationshipAction}
      handleMuteAction={shellActions.handleMuteAction}
      handleOpenOriginalTopic={shellActions.handleOpenOriginalTopic}
      columnMode
    />
  );
  const activateWorkspaceColumn = async (column: ColumnState) => {
    if (
      column.scope &&
      (column.scope.topicId !== activeTopic ||
        column.scope.channelId !== (selectedChannelIdByTopic[activeTopic] ?? null))
    ) {
      await syncTopicContext(column.scope.topicId, column.scope.channelId);
    }
    if (column.kind === 'thread' && column.entityId) {
      await openThread(column.entityId, { topic: column.scope?.topicId });
      return;
    }
    if (column.kind === 'profile' && column.entityId) {
      await openAuthorDetail(column.entityId);
      return;
    }
    if (column.kind === 'conversation' && column.entityId) {
      await openDirectMessagePane(column.entityId);
      return;
    }
    if (column.kind === 'stream') {
      setSelectedLiveSessionId(column.entityId ?? null);
      setSelectedGameRoomId(null);
      focusPrimarySection('live');
      return;
    }
    if (column.kind === 'game' || column.kind === 'metaverse') {
      setSelectedGameRoomId(column.entityId ?? null);
      setSelectedLiveSessionId(null);
      focusPrimarySection('game');
      return;
    }
    const sectionByKind: Partial<Record<ColumnKind, PrimarySection>> = {
      timeline: 'timeline',
      notifications: 'notifications',
      explore: 'explore',
      messages: 'messages',
      profile: 'profile',
    };
    const section = sectionByKind[column.kind];
    if (section) focusPrimarySection(section);
  };
  const columnTitles: Record<ColumnKind, string> = {
    timeline: t('shell:primarySections.timeline'),
    notifications: t('shell:primarySections.notifications'),
    thread: t('shell:context.thread'),
    profile: t('shell:primarySections.profile'),
    explore: t('shell:primarySections.explore'),
    messages: t('shell:primarySections.messages'),
    conversation: 'Conversation',
    stream: t('shell:primarySections.live'),
    game: t('shell:primarySections.game'),
    metaverse: 'Metaverse',
  };
  const workspace = (
    <DesktopShellColumnWorkspace
      scopeLabel={viewModels.activeComposeAudienceLabel}
      activeTimelineView={shellChromeState.timelineView}
      locale={locale}
      mentionCandidates={viewModels.mentionCandidates}
      onColumnAttachmentSelection={shellActions.handleColumnDraftAttachmentSelection}
      onRemoveColumnAttachment={shellActions.handleRemoveColumnDraftAttachment}
      onSubmitColumnDraft={shellActions.handleSubmitColumnDraft}
      onEndLiveSession={shellActions.handleEndLiveSession}
      onJoinLiveSession={shellActions.handleJoinLiveSession}
      onLeaveLiveSession={shellActions.handleLeaveLiveSession}
      onOpenGameCreate={() => dialogs.setGameCreateDialogOpen(true)}
      onOpenLiveCreate={() => dialogs.setLiveCreateDialogOpen(true)}
      timelineViewItems={viewModels.timelineViewItems}
      onSelectTimelineView={focusTimelineView}
      onActivateColumn={(column) => void activateWorkspaceColumn(column)}
      renderPrimarySurface={renderPrimarySurface}
      messagesSurface={renderMessagesSurface('messages')}
      renderConversationSurface={(column) =>
        renderMessagesSurface('conversation', column.entityId)
      }
      notificationsSurface={notificationsSurface}
      renderThreadSurface={(column) =>
        renderDetailSurface('thread', column.entityId, column.scope?.topicId)
      }
      renderProfileSurface={(column) =>
        renderDetailSurface('profile', column.entityId, column.scope?.topicId)
      }
      titles={columnTitles}
    />
  );

  return (
    <>
      <ShellFrame
        skipTargetId={SHELL_WORKSPACE_ID}
        workspace={workspace}
        workspaceLayout='column'
        globalControls={
          <DesktopShellControlCenter
            triggerRef={controlCenterTriggerRef}
            topicItems={topicNavItems}
            topicInput={topicInput}
            titles={columnTitles}
            updateAvailable={updateAvailable}
            onTopicInputChange={setTopicInput}
            onAddTopic={shellActions.handleAddTopic}
            onOpenChannelManager={() => dialogs.setChannelDialogOpen(true)}
            onActivateColumn={activateWorkspaceColumn}
            onOpenSettings={handleOpenSettingsSection}
            onSelectTopic={(topic) => void shellActions.handleSelectTopic(topic)}
            onSelectChannel={(topic, channelId) => {
              shellActions.handleSelectPrivateChannel(topic, channelId);
            }}
            onOpenChannelSettings={(topic, channelId) => {
              setInviteOutput(null);
              setChannelError(null);
              shellActions.handleSelectPrivateChannel(topic, channelId);
              dialogs.setChannelSettingsDialogOpen(true);
            }}
            onLeaveChannel={(topic, channelId) => dialogs.openLeaveChannelDialog(topic, channelId)}
            onRemoveTopic={(topic) => void shellActions.handleRemoveTopic(topic)}
            onCopyTopicLink={(topic) => handleCopyInternalLink(buildTopicLink(topic))}
            onRequestTopicIndexing={(topic) =>
              setIndexingTarget({ kind: 'public_topic', topicId: topic })
            }
            onToggleTopicGossip={(topic, enabled) =>
              void shellActions.handleToggleTopicGossip(topic, enabled)
            }
            onToggleChannelGossip={(topic, channelId, enabled) =>
              void shellActions.handleToggleChannelGossip(topic, channelId, enabled)
            }
          />
        }
      />

      <DesktopShellOverlays
        actions={shellActions}
        dialogs={dialogs}
        t={t}
        viewModels={viewModels}
        handleCopyInternalLink={handleCopyInternalLink}
        sharePreview={sharePreview}
        clipboardToastId={clipboardToastId}
        onRequestPrivateIndexing={setIndexingTarget}
      />

      <CommunityIndexingRequestDialog
        api={api}
        target={indexingTarget}
        eligibleNodeBaseUrls={eligibleIndexingNodes}
        onOpenChange={(open) => {
          if (!open) setIndexingTarget(null);
        }}
        onOpenCommunityNodeSettings={handleOpenCommunityNodeSettings}
      />

      <DesktopShellSettingsDrawer
        api={api}
        onThemeChange={onThemeChange}
        onLocaleChange={(nextLocale) => {
          void i18nInstance.changeLanguage(nextLocale);
        }}
        syncRoute={syncRoute}
        setSettingsOpen={setSettingsOpen}
        viewModels={viewModels}
        handleImportPeer={shellActions.handleImportPeer}
        handleSaveDiscoverySeeds={shellActions.handleSaveDiscoverySeeds}
        handleSaveCommunityNodes={shellActions.handleSaveCommunityNodes}
        handleClearCommunityNodes={shellActions.handleClearCommunityNodes}
        handleAuthenticateCommunityNode={shellActions.handleAuthenticateCommunityNode}
        handleSetCommunityNodeInviteCode={shellActions.handleSetCommunityNodeInviteCode}
        handleFetchCommunityNodeConsents={shellActions.handleFetchCommunityNodeConsents}
        handleAcceptCommunityNodeConsents={shellActions.handleAcceptCommunityNodeConsents}
        handleRefreshCommunityNode={shellActions.handleRefreshCommunityNode}
        handleClearCommunityNodeToken={shellActions.handleClearCommunityNodeToken}
        handleCreateCustomReactionAsset={shellActions.handleCreateCustomReactionAsset}
        handleRemoveBookmarkedCustomReaction={shellActions.handleRemoveBookmarkedCustomReaction}
      />
    </>
  );
}
