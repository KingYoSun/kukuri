import { startTransition, useCallback, useRef } from 'react';

import type { DesktopApi } from '@/lib/api';
import type { LoadNotificationsSection } from '@/shell/data/loaders/useNotificationLoaders';
import { VISIBLE_TIMELINE_LIMIT } from '@/shell/pagination';
import {
  authorViewFromDirectMessageConversation,
  communityNodesToDraftNodes,
  mergeKnownAuthors,
  messageFromError,
  privateTimelineScope,
  profileInputFromProfile,
  seedPeersToEditorValue,
} from '@/shell/presentation';
import { setRecordEntry } from '@/shell/stateUpdates';
import {
  timelineStorageKeyForChannel,
  useDesktopShellFieldSetter,
  type DesktopShellStoreApi,
} from '@/shell/store';
import {
  activeWorkspaceColumn,
  activeWorkspaceScope,
  primarySectionForColumn,
} from '@/shell/slices/workspace';

type UseDesktopShellSectionLoadersArgs = {
  api: DesktopApi;
  loadReactionCatalogData: () => Promise<void>;
  loadNotificationsSection: LoadNotificationsSection;
  storeApi: DesktopShellStoreApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
};

export function useDesktopShellSectionLoaders({
  api,
  loadReactionCatalogData,
  loadNotificationsSection,
  storeApi,
  translate,
}: UseDesktopShellSectionLoadersArgs) {
  const profileRequestId = useRef(0);
  const communityNodeRequestId = useRef(0);
  const authorRequestIds = useRef(new Map<string, number>());
  const setAuthorError = useDesktopShellFieldSetter('authorError');
  const setAuthorErrorsByPubkey = useDesktopShellFieldSetter('authorErrorsByPubkey');
  const setAuthorTimelinesByPubkey = useDesktopShellFieldSetter('authorTimelinesByPubkey');
  const setAuthorTimelineNextCursorByPubkey = useDesktopShellFieldSetter(
    'authorTimelineNextCursorByPubkey'
  );
  const setBookmarkedPosts = useDesktopShellFieldSetter('bookmarkedPosts');
  const setCommunityNodeConfig = useDesktopShellFieldSetter('communityNodeConfig');
  const setCommunityNodeError = useDesktopShellFieldSetter('communityNodeError');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setCommunityNodeManifests = useDesktopShellFieldSetter('communityNodeManifests');
  const setDirectMessageError = useDesktopShellFieldSetter('directMessageError');
  const setDirectMessages = useDesktopShellFieldSetter('directMessages');
  const setDirectMessageStatusByPeer = useDesktopShellFieldSetter('directMessageStatusByPeer');
  const setDirectMessageTimelineByPeer = useDesktopShellFieldSetter(
    'directMessageTimelineByPeer'
  );
  const setDirectMessageTimelineNextCursorByPeer = useDesktopShellFieldSetter(
    'directMessageTimelineNextCursorByPeer'
  );
  const setDiscoveryConfig = useDesktopShellFieldSetter('discoveryConfig');
  const setDiscoveryError = useDesktopShellFieldSetter('discoveryError');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setGamePanelStateByScopeKey = useDesktopShellFieldSetter('gamePanelStateByScopeKey');
  const setGameRoomsByScopeKey = useDesktopShellFieldSetter('gameRoomsByScopeKey');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const setLivePanelStateByScopeKey = useDesktopShellFieldSetter('livePanelStateByScopeKey');
  const setLiveSessionsByScopeKey = useDesktopShellFieldSetter('liveSessionsByScopeKey');
  const setLocalPeerTicket = useDesktopShellFieldSetter('localPeerTicket');
  const setLocalProfile = useDesktopShellFieldSetter('localProfile');
  const setProfileDraft = useDesktopShellFieldSetter('profileDraft');
  const setProfileError = useDesktopShellFieldSetter('profileError');
  const setProfilePanelState = useDesktopShellFieldSetter('profilePanelState');
  const setProfileTimeline = useDesktopShellFieldSetter('profileTimeline');
  const setProfileTimelineNextCursor = useDesktopShellFieldSetter(
    'profileTimelineNextCursor'
  );
  const setReactionPanelState = useDesktopShellFieldSetter('reactionPanelState');
  const setSelectedAuthor = useDesktopShellFieldSetter('selectedAuthor');
  const setSelectedAuthorTimeline = useDesktopShellFieldSetter('selectedAuthorTimeline');
  const setSelectedAuthorTimelineNextCursor = useDesktopShellFieldSetter(
    'selectedAuthorTimelineNextCursor'
  );
  const setSocialConnections = useDesktopShellFieldSetter('socialConnections');
  const setSocialConnectionsPanelState = useDesktopShellFieldSetter(
    'socialConnectionsPanelState'
  );

  const loadLiveSection = useCallback(
    async (topic: string, selectedChannelId: string | null) => {
      const scopeKey = timelineStorageKeyForChannel(topic, selectedChannelId);
      try {
        const sessions = await api.listLiveSessions(
          topic,
          privateTimelineScope(selectedChannelId)
        );
        startTransition(() => {
          setLiveSessionsByScopeKey(setRecordEntry(scopeKey, sessions));
          setLivePanelStateByScopeKey(
            setRecordEntry(scopeKey, { status: 'ready', error: null })
          );
        });
      } catch (error) {
        const panelState = {
          status: 'error' as const,
          error: messageFromError(
            error,
            translate('common:errors.failedToLoadLiveSessions')
          ),
        };
        setLivePanelStateByScopeKey(setRecordEntry(scopeKey, panelState));
      }
    },
    [
      api,
      setLivePanelStateByScopeKey,
      setLiveSessionsByScopeKey,
      translate,
    ]
  );

  const loadGameSection = useCallback(
    async (topic: string, selectedChannelId: string | null) => {
      const scopeKey = timelineStorageKeyForChannel(topic, selectedChannelId);
      try {
        const rooms = await api.listGameRooms(topic, privateTimelineScope(selectedChannelId));
        startTransition(() => {
          setGameRoomsByScopeKey(setRecordEntry(scopeKey, rooms));
          setGamePanelStateByScopeKey(
            setRecordEntry(scopeKey, { status: 'ready', error: null })
          );
        });
      } catch (error) {
        const panelState = {
          status: 'error' as const,
          error: messageFromError(error, translate('common:errors.failedToLoadGameRooms')),
        };
        setGamePanelStateByScopeKey(setRecordEntry(scopeKey, panelState));
      }
    },
    [
      api,
      setGamePanelStateByScopeKey,
      setGameRoomsByScopeKey,
      translate,
    ]
  );

  const loadProfileSection = useCallback(async () => {
    const requestId = ++profileRequestId.current;
    setProfileError(null);
    setProfilePanelState({ status: 'loading', error: null });
    try {
      const [profile, following, followed, muted, blocking] = await Promise.all([
        api.getMyProfile(),
        api.listSocialConnections('following'),
        api.listSocialConnections('followed'),
        api.listSocialConnections('muted'),
        api.listSocialConnections('blocking'),
      ]);
      const timeline = await api.listProfileTimeline(
        profile.pubkey,
        null,
        VISIBLE_TIMELINE_LIMIT
      );
      if (requestId !== profileRequestId.current) return;
      startTransition(() => {
        setLocalProfile(profile);
        if (!storeApi.getState().profileDirty) {
          setProfileDraft(profileInputFromProfile(profile));
        }
        setProfileTimeline(timeline.items);
        setProfileTimelineNextCursor(timeline.next_cursor ?? null);
        setProfileError(null);
        setProfilePanelState({ status: 'ready', error: null });
        setSocialConnections({ following, followed, muted, blocking });
        setKnownAuthorsByPubkey((current) =>
          mergeKnownAuthors(current, [...following, ...followed, ...muted, ...blocking])
        );
        setSocialConnectionsPanelState({ status: 'ready', error: null });
      });
    } catch (error) {
      if (requestId !== profileRequestId.current) return;
      const message = messageFromError(
        error,
        translate('common:errors.failedToLoadProfile')
      );
      setProfileError(message);
      setProfilePanelState({ status: 'error', error: message });
    }
  }, [
    api,
    setKnownAuthorsByPubkey,
    setLocalProfile,
    setProfileDraft,
    setProfileError,
    setProfilePanelState,
    setProfileTimeline,
    setProfileTimelineNextCursor,
    setSocialConnections,
    setSocialConnectionsPanelState,
    storeApi,
    translate,
  ]);

  const loadAuthorSection = useCallback(
    async (pubkey: string) => {
      const requestId = (authorRequestIds.current.get(pubkey) ?? 0) + 1;
      authorRequestIds.current.set(pubkey, requestId);
      try {
        const [author, timeline] = await Promise.all([
          api.getAuthorSocialView(pubkey),
          api.listProfileTimeline(pubkey, null, VISIBLE_TIMELINE_LIMIT),
        ]);
        if (requestId !== authorRequestIds.current.get(pubkey)) return;
        startTransition(() => {
          if (storeApi.getState().selectedAuthorPubkey === pubkey) {
            setSelectedAuthor(author);
            setSelectedAuthorTimeline(timeline.items);
            setSelectedAuthorTimelineNextCursor(timeline.next_cursor ?? null);
            setAuthorError(null);
          }
          setAuthorTimelinesByPubkey(setRecordEntry(pubkey, timeline.items));
          setAuthorTimelineNextCursorByPubkey(
            setRecordEntry(pubkey, timeline.next_cursor ?? null)
          );
          setAuthorErrorsByPubkey(setRecordEntry(pubkey, null));
          if (author) {
            setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [author]));
          }
        });
      } catch (error) {
        if (requestId !== authorRequestIds.current.get(pubkey)) return;
        const message = messageFromError(error, translate('common:errors.failedToLoadAuthor'));
        if (storeApi.getState().selectedAuthorPubkey === pubkey) setAuthorError(message);
        setAuthorErrorsByPubkey(setRecordEntry(pubkey, message));
      }
    },
    [
      api,
      setAuthorError,
      setAuthorErrorsByPubkey,
      setAuthorTimelinesByPubkey,
      setAuthorTimelineNextCursorByPubkey,
      setKnownAuthorsByPubkey,
      setSelectedAuthor,
      setSelectedAuthorTimeline,
      setSelectedAuthorTimelineNextCursor,
      storeApi,
      translate,
    ]
  );

  const loadMessagesSection = useCallback(async () => {
    try {
      const directMessages = await api.listDirectMessages();
      startTransition(() => {
        setDirectMessages(directMessages);
        setKnownAuthorsByPubkey((current) =>
          mergeKnownAuthors(
            current,
            directMessages.map(authorViewFromDirectMessageConversation)
          )
        );
      });
      const selectedPeerPubkey = storeApi.getState().selectedDirectMessagePeerPubkey;
      if (!selectedPeerPubkey) {
        setDirectMessageError(null);
        return;
      }
      const [timelineResult, statusResult] = await Promise.allSettled([
        api.listDirectMessageMessages(selectedPeerPubkey, null, VISIBLE_TIMELINE_LIMIT),
        api.getDirectMessageStatus(selectedPeerPubkey),
      ]);
      startTransition(() => {
        if (timelineResult.status === 'fulfilled') {
          setDirectMessageTimelineByPeer(
            setRecordEntry(selectedPeerPubkey, timelineResult.value.items)
          );
          setDirectMessageTimelineNextCursorByPeer(
            setRecordEntry(selectedPeerPubkey, timelineResult.value.next_cursor ?? null)
          );
        }
        if (statusResult.status === 'fulfilled') {
          setDirectMessageStatusByPeer(
            setRecordEntry(selectedPeerPubkey, statusResult.value)
          );
        }
        setDirectMessageError(
          timelineResult.status === 'fulfilled' && statusResult.status === 'fulfilled'
            ? null
            : messageFromError(
                timelineResult.status === 'rejected'
                  ? timelineResult.reason
                  : statusResult.status === 'rejected'
                    ? statusResult.reason
                    : null,
                  translate('common:errors.failedToLoadDirectMessages')
              )
        );
      });
    } catch (error) {
      setDirectMessageError(
        messageFromError(error, translate('common:errors.failedToLoadDirectMessages'))
      );
    }
  }, [
    api,
    setDirectMessageError,
    setDirectMessages,
    setDirectMessageStatusByPeer,
    setDirectMessageTimelineByPeer,
    setDirectMessageTimelineNextCursorByPeer,
    setKnownAuthorsByPubkey,
    storeApi,
    translate,
  ]);

  const loadBookmarksSection = useCallback(async () => {
    try {
      setBookmarkedPosts(await api.listBookmarkedPosts());
    } catch {
      // best effort refresh
    }
  }, [api, setBookmarkedPosts]);

  const loadCommunityIndexCapability = useCallback(async () => {
    const requestId = ++communityNodeRequestId.current;
    const previousConfig = storeApi.getState().communityNodeConfig;
    try {
      const config = await api.getCommunityNodeConfig();
      if (requestId !== communityNodeRequestId.current ||
        storeApi.getState().communityNodeConfig !== previousConfig) return;
      startTransition(() => {
        setCommunityNodeConfig(config);
        if (!storeApi.getState().communityNodeEditorDirty) {
          setCommunityNodeInput(communityNodesToDraftNodes(config));
        }
        setCommunityNodeError(null);
      });
      const baseUrls = config.nodes
        .map((node) => node.base_url)
        .filter((baseUrl) => baseUrl.trim().length > 0);
      if (baseUrls.length === 0) {
        setCommunityNodeManifests({});
        return;
      }
      setCommunityNodeManifests((current) => {
        const next = { ...current };
        for (const baseUrl of baseUrls) {
          // A refresh is background work when a usable manifest is already on screen.
          // Keep that successful value until the replacement settles so consumers do not
          // briefly project an empty/no-node state while another Column is activated.
          if (next[baseUrl]?.status !== 'ok') {
            next[baseUrl] = { status: 'loading' };
          }
        }
        return next;
      });
      const manifestResults = await Promise.all(
        baseUrls.map(async (baseUrl) => {
          try {
            const result = await api.fetchCommunityNodeManifest(baseUrl);
            return [
              baseUrl,
              result.status === 'ok' && result.manifest
                ? { status: 'ok' as const, manifest: result.manifest }
                : { status: 'absent' as const },
            ] as const;
          } catch (error) {
            return [
              baseUrl,
              {
                status: 'error' as const,
                error: messageFromError(error, translate('common:errors.failedToLoadSettings')),
              },
            ] as const;
          }
        })
      );
      if (requestId !== communityNodeRequestId.current) return;
      const currentUrls = storeApi.getState().communityNodeConfig.nodes.map((node) => node.base_url);
      if (JSON.stringify(currentUrls) !== JSON.stringify(baseUrls)) return;
      const manifests = Object.fromEntries(manifestResults);
      setCommunityNodeManifests((current) => ({ ...current, ...manifests }));
    } catch (error) {
      if (requestId !== communityNodeRequestId.current ||
        storeApi.getState().communityNodeConfig !== previousConfig) return;
      storeApi.getState().patchState({ communityNodeConfigError: 'config_unavailable' });
      setCommunityNodeError(
        messageFromError(error, translate('common:errors.failedToLoadSettings'))
      );
    }
  }, [
    api,
    setCommunityNodeConfig,
    setCommunityNodeError,
    setCommunityNodeInput,
    setCommunityNodeManifests,
    storeApi,
    translate,
  ]);

  const loadSettingsSection = useCallback(async () => {
    const { activeSettingsSection } = storeApi.getState().shellChromeState;
    if (activeSettingsSection === 'connectivity') {
      try {
        setLocalPeerTicket(await api.getLocalPeerTicket());
      } catch {
        // best effort refresh
      }
      return;
    }
    if (activeSettingsSection === 'discovery') {
      try {
        const config = await api.getDiscoveryConfig();
        setDiscoveryConfig(config);
        if (!storeApi.getState().discoveryEditorDirty) {
          setDiscoverySeedInput(seedPeersToEditorValue(config));
        }
        setDiscoveryError(null);
      } catch (error) {
        setDiscoveryError(
          messageFromError(error, translate('common:errors.failedToLoadSettings'))
        );
      }
      return;
    }
    if (activeSettingsSection === 'community-node') {
      await loadCommunityIndexCapability();
      return;
    }
    if (activeSettingsSection === 'reactions') {
      try {
        const [bookmarkedPosts] = await Promise.all([
          api.listBookmarkedPosts(),
          loadReactionCatalogData(),
        ]);
        startTransition(() => setBookmarkedPosts(bookmarkedPosts));
      } catch (error) {
        setReactionPanelState({
          status: 'error',
          error: messageFromError(error, translate('common:errors.failedToLoadSettings')),
        });
      }
    }
  }, [
    api,
    loadReactionCatalogData,
    setBookmarkedPosts,
    loadCommunityIndexCapability,
    setDiscoveryConfig,
    setDiscoveryError,
    setDiscoverySeedInput,
    setLocalPeerTicket,
    setReactionPanelState,
    storeApi,
    translate,
  ]);

  const loadShellSections = useCallback(
    async (topic: string) => {
      const state = storeApi.getState();
      const activeColumn = activeWorkspaceColumn(state.workspaceState);
      const activeScope = activeWorkspaceScope(state.workspaceState);
      const selectedChannelId = activeScope.topicId === topic ? activeScope.channelId : null;
      const selectedAuthorPubkey = state.selectedAuthorPubkey;
      const { settingsOpen } = state.shellChromeState;
      const activePrimarySection = primarySectionForColumn(activeColumn);
      const timelineView = activeColumn.kind === 'timeline' ? activeColumn.timelineView ?? 'feed' : 'feed';
      const tasks: Promise<void>[] = [];
      tasks.push(loadCommunityIndexCapability());

      if (activePrimarySection === 'live') {
        tasks.push(loadLiveSection(topic, selectedChannelId));
      }
      if (activePrimarySection === 'game') {
        tasks.push(loadGameSection(topic, selectedChannelId));
      }
      if (state.workspaceState.columns.some((column) => column.kind === 'profile' && !column.entityId)) {
        tasks.push(loadProfileSection());
      }
      if (selectedAuthorPubkey) {
        tasks.push(loadAuthorSection(selectedAuthorPubkey));
      }
      if (activePrimarySection === 'messages' || state.directMessagePaneOpen) {
        tasks.push(loadMessagesSection());
      }
      if (activePrimarySection === 'notifications') {
        tasks.push(loadNotificationsSection());
      }
      if (
        (activePrimarySection === 'timeline' && timelineView === 'bookmarks') ||
        // 非 active な Timeline Column が Bookmarks を表示している場合もロードする(Issue #765)。
        state.workspaceState.columns.some((column) => column.timelineView === 'bookmarks')
      ) {
        tasks.push(loadBookmarksSection());
      }
      if (settingsOpen) {
        tasks.push(loadSettingsSection());
      }
      await Promise.allSettled(tasks);
    },
    [
      loadAuthorSection,
      loadBookmarksSection,
      loadGameSection,
      loadLiveSection,
      loadCommunityIndexCapability,
      loadMessagesSection,
      loadNotificationsSection,
      loadProfileSection,
      loadSettingsSection,
      storeApi,
    ]
  );

  // 個別 loader も公開する: section 遷移起点の effect(useDesktopShellDataEffects)が
  // 同じ実装を呼ぶための入口。通知の取得・state反映は注入したloaderへ委譲する。
  return {
    loadShellSections,
    loadProfileSection,
    loadAuthorSection,
    loadMessagesSection,
    loadNotificationsSection,
    loadCommunityIndexCapability,
  };
}
