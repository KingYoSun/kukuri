import { startTransition, useCallback } from 'react';

import type { CommunityNodeNodeStatus, DesktopApi, NotificationView } from '@/lib/api';
import {
  eligibleCommunityIndexNodes,
  resolveCommunityIndexNodeBaseUrl,
} from '@/lib/api/communityIndex';
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
  useDesktopShellFieldSetter,
  type DesktopShellStoreApi,
} from '@/shell/store';

type UseDesktopShellSectionLoadersArgs = {
  api: DesktopApi;
  loadReactionCatalogData: () => Promise<void>;
  storeApi: DesktopShellStoreApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
};

export function useDesktopShellSectionLoaders({
  api,
  loadReactionCatalogData,
  storeApi,
  translate,
}: UseDesktopShellSectionLoadersArgs) {
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
  const setCommunityIndexNodeBaseUrl = useDesktopShellFieldSetter('communityIndexNodeBaseUrl');
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
  const setGamePanelStateByTopic = useDesktopShellFieldSetter('gamePanelStateByTopic');
  const setGameRoomsByTopic = useDesktopShellFieldSetter('gameRoomsByTopic');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');
  const setLivePanelStateByTopic = useDesktopShellFieldSetter('livePanelStateByTopic');
  const setLiveSessionsByTopic = useDesktopShellFieldSetter('liveSessionsByTopic');
  const setLocalPeerTicket = useDesktopShellFieldSetter('localPeerTicket');
  const setLocalProfile = useDesktopShellFieldSetter('localProfile');
  const setNotificationAutoReadError = useDesktopShellFieldSetter(
    'notificationAutoReadError'
  );
  const setNotificationPanelState = useDesktopShellFieldSetter('notificationPanelState');
  const setNotifications = useDesktopShellFieldSetter('notifications');
  const setNotificationStatus = useDesktopShellFieldSetter('notificationStatus');
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
      try {
        const sessions = await api.listLiveSessions(
          topic,
          privateTimelineScope(selectedChannelId)
        );
        startTransition(() => {
          setLiveSessionsByTopic(setRecordEntry(topic, sessions));
          setLivePanelStateByTopic(
            setRecordEntry(topic, { status: 'ready', error: null })
          );
        });
      } catch (error) {
        setLivePanelStateByTopic(
          setRecordEntry(topic, {
            status: 'error',
            error: messageFromError(
              error,
              translate('common:errors.failedToLoadLiveSessions')
            ),
          })
        );
      }
    },
    [api, setLivePanelStateByTopic, setLiveSessionsByTopic, translate]
  );

  const loadGameSection = useCallback(
    async (topic: string, selectedChannelId: string | null) => {
      try {
        const rooms = await api.listGameRooms(topic, privateTimelineScope(selectedChannelId));
        startTransition(() => {
          setGameRoomsByTopic(setRecordEntry(topic, rooms));
          setGamePanelStateByTopic(
            setRecordEntry(topic, { status: 'ready', error: null })
          );
        });
      } catch (error) {
        setGamePanelStateByTopic(
          setRecordEntry(topic, {
            status: 'error',
            error: messageFromError(error, translate('common:errors.failedToLoadGameRooms')),
          })
        );
      }
    },
    [api, setGamePanelStateByTopic, setGameRoomsByTopic, translate]
  );

  const loadProfileSection = useCallback(async () => {
    try {
      const [profile, following, followed, muted] = await Promise.all([
        api.getMyProfile(),
        api.listSocialConnections('following'),
        api.listSocialConnections('followed'),
        api.listSocialConnections('muted'),
      ]);
      const timeline = await api.listProfileTimeline(
        profile.pubkey,
        null,
        VISIBLE_TIMELINE_LIMIT
      );
      startTransition(() => {
        setLocalProfile(profile);
        if (!storeApi.getState().profileDirty) {
          setProfileDraft(profileInputFromProfile(profile));
        }
        setProfileTimeline(timeline.items);
        setProfileTimelineNextCursor(timeline.next_cursor ?? null);
        setProfileError(null);
        setProfilePanelState({ status: 'ready', error: null });
        setSocialConnections({ following, followed, muted });
        setKnownAuthorsByPubkey((current) =>
          mergeKnownAuthors(current, [...following, ...followed, ...muted])
        );
        setSocialConnectionsPanelState({ status: 'ready', error: null });
      });
    } catch (error) {
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
      try {
        const [author, timeline] = await Promise.all([
          api.getAuthorSocialView(pubkey),
          api.listProfileTimeline(pubkey, null, VISIBLE_TIMELINE_LIMIT),
        ]);
        startTransition(() => {
          setSelectedAuthor(author);
          setSelectedAuthorTimeline(timeline.items);
          setSelectedAuthorTimelineNextCursor(timeline.next_cursor ?? null);
          setAuthorTimelinesByPubkey(setRecordEntry(pubkey, timeline.items));
          setAuthorTimelineNextCursorByPubkey(
            setRecordEntry(pubkey, timeline.next_cursor ?? null)
          );
          setAuthorError(null);
          setAuthorErrorsByPubkey(setRecordEntry(pubkey, null));
          if (author) {
            setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [author]));
          }
        });
      } catch (error) {
        const message = messageFromError(error, translate('common:errors.failedToLoadAuthor'));
        setAuthorError(message);
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

  const loadNotificationsSection = useCallback(async () => {
    try {
      const [status, notificationItems] = await Promise.all([
        api.getNotificationStatus(),
        api.listNotifications(),
      ]);
      let nextNotifications: NotificationView[] = notificationItems;
      let nextStatus = status;
      if (notificationItems.some((notification) => !notification.read_at)) {
        try {
          nextStatus = await api.markAllNotificationsRead();
          const readAt = Date.now();
          nextNotifications = notificationItems.map((notification) =>
            notification.read_at ? notification : { ...notification, read_at: readAt }
          );
          setNotificationAutoReadError(null);
        } catch (notificationReadError) {
          setNotificationAutoReadError(
            messageFromError(
              notificationReadError,
              translate('shell:notifications.errors.failedAutoRead')
            )
          );
        }
      }
      startTransition(() => {
        setNotificationStatus(nextStatus);
        setNotifications(nextNotifications);
        setNotificationPanelState({ status: 'ready', error: null });
      });
    } catch (error) {
      setNotificationPanelState({
        status: 'error',
        error: messageFromError(
          error,
          translate('shell:notifications.errors.failedToLoad')
        ),
      });
    }
  }, [
    api,
    setNotificationAutoReadError,
    setNotificationPanelState,
    setNotifications,
    setNotificationStatus,
    translate,
  ]);

  const loadBookmarksSection = useCallback(async () => {
    try {
      setBookmarkedPosts(await api.listBookmarkedPosts());
    } catch {
      // best effort refresh
    }
  }, [api, setBookmarkedPosts]);

  const loadCommunityIndexCapability = useCallback(async (
    refreshedStatuses?: readonly CommunityNodeNodeStatus[]
  ) => {
    try {
      const config = await api.getCommunityNodeConfig();
      const statuses = refreshedStatuses ?? storeApi.getState().communityNodeStatuses;
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
        setCommunityIndexNodeBaseUrl(null);
        return;
      }
      setCommunityNodeManifests((current) => {
        const next = { ...current };
        for (const baseUrl of baseUrls) {
          next[baseUrl] = { status: 'loading' };
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
      const manifests = Object.fromEntries(manifestResults);
      setCommunityNodeManifests((current) => ({ ...current, ...manifests }));
      const eligible = eligibleCommunityIndexNodes(config, statuses, manifests);
      setCommunityIndexNodeBaseUrl((current) =>
        resolveCommunityIndexNodeBaseUrl(current, eligible)
      );
    } catch (error) {
      setCommunityNodeError(
        messageFromError(error, translate('common:errors.failedToLoadSettings'))
      );
      setCommunityIndexNodeBaseUrl(null);
    }
  }, [
    api,
    setCommunityIndexNodeBaseUrl,
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
      const selectedChannelId = state.selectedChannelIdByTopic[topic] ?? null;
      const selectedAuthorPubkey = state.selectedAuthorPubkey;
      const { activePrimarySection, settingsOpen, timelineView } = state.shellChromeState;
      const tasks: Promise<void>[] = [];
      tasks.push(loadCommunityIndexCapability());

      if (activePrimarySection === 'live') {
        tasks.push(loadLiveSection(topic, selectedChannelId));
      }
      if (activePrimarySection === 'game') {
        tasks.push(loadGameSection(topic, selectedChannelId));
      }
      if (activePrimarySection === 'profile') {
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
      if (activePrimarySection === 'timeline' && timelineView === 'bookmarks') {
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
  // 同じ実装を呼ぶための SSoT。取得・state 反映ロジックはこのファイルにだけ置く。
  return {
    loadShellSections,
    loadProfileSection,
    loadAuthorSection,
    loadMessagesSection,
    loadNotificationsSection,
    loadCommunityIndexCapability,
  };
}
