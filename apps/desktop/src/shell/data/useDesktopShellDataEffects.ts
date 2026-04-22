import {
  startTransition,
  useEffect,
  type MutableRefObject,
} from 'react';

import type {
  AttachmentView,
  DesktopApi,
} from '@/lib/api';

import {
  createObjectUrlFromPayload,
  logMediaDebug,
} from '@/shell/media';
import {
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  REFRESH_INTERVAL_MS,
  STATUS_REFRESH_INTERVAL_MS,
  type DesktopShellState,
  type DesktopShellStateValue,
  type DesktopShellStoreApi,
} from '@/shell/store';
import { VISIBLE_TIMELINE_LIMIT } from '@/shell/pagination';
import {
  authorViewFromDirectMessageConversation,
  createGameEditorDraft,
  mergeKnownAuthors,
  messageFromError,
  profileInputFromProfile,
} from '@/shell/selectors';

type Setter<K extends keyof DesktopShellState> = (
  value: DesktopShellStateValue<K>
) => void;

type UseDesktopShellDataEffectsArgs = {
  api: DesktopApi;
  translate: (key: string, options?: Record<string, unknown>) => string;
  storeApi: DesktopShellStoreApi;
  trackedTopics: string[];
  activeTopic: string;
  selectedThread: string | null;
  activeGameRooms: DesktopShellState['gameRoomsByTopic'][string];
  activeJoinedChannels: DesktopShellState['joinedChannelsByTopic'][string];
  selectedPrivateChannelId: string | null;
  mediaObjectUrls: DesktopShellState['mediaObjectUrls'];
  shellChromeState: DesktopShellState['shellChromeState'];
  selectedAuthorPubkey: string | null;
  previewableMediaAttachments: AttachmentView[];
  remoteObjectUrlRef: MutableRefObject<Map<string, string>>;
  draftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  directMessageDraftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  mediaFetchAttemptRef: MutableRefObject<Map<string, number>>;
  visibleRefreshInFlightRef: MutableRefObject<boolean>;
  loadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>;
  refreshVisibleShellData: (
    topic: string,
    currentThread: string | null,
    mode?: 'apply' | 'buffer'
  ) => Promise<void>;
  setNotificationStatus: Setter<'notificationStatus'>;
  setLocalProfile: Setter<'localProfile'>;
  setProfileDraft: Setter<'profileDraft'>;
  setKnownAuthorsByPubkey: Setter<'knownAuthorsByPubkey'>;
  setProfileTimeline: Setter<'profileTimeline'>;
  setProfileTimelineNextCursor: Setter<'profileTimelineNextCursor'>;
  setProfileError: Setter<'profileError'>;
  setProfilePanelState: Setter<'profilePanelState'>;
  setSocialConnections: Setter<'socialConnections'>;
  setSocialConnectionsPanelState: Setter<'socialConnectionsPanelState'>;
  setSelectedAuthor: Setter<'selectedAuthor'>;
  setSelectedAuthorTimeline: Setter<'selectedAuthorTimeline'>;
  setSelectedAuthorTimelineNextCursor: Setter<'selectedAuthorTimelineNextCursor'>;
  setAuthorError: Setter<'authorError'>;
  setDirectMessages: Setter<'directMessages'>;
  setDirectMessageTimelineByPeer: Setter<'directMessageTimelineByPeer'>;
  setDirectMessageTimelineNextCursorByPeer: Setter<'directMessageTimelineNextCursorByPeer'>;
  setDirectMessageStatusByPeer: Setter<'directMessageStatusByPeer'>;
  setDirectMessageError: Setter<'directMessageError'>;
  setNotifications: Setter<'notifications'>;
  setNotificationPanelState: Setter<'notificationPanelState'>;
  setNotificationAutoReadError: Setter<'notificationAutoReadError'>;
  setGameDrafts: Setter<'gameDrafts'>;
  setSelectedChannelIdByTopic: Setter<'selectedChannelIdByTopic'>;
  setComposeChannelByTopic: Setter<'composeChannelByTopic'>;
  setTimelineScopeByTopic: Setter<'timelineScopeByTopic'>;
  setMediaObjectUrls: Setter<'mediaObjectUrls'>;
};

export function useDesktopShellDataEffects({
  api,
  translate,
  storeApi,
  trackedTopics,
  activeTopic,
  selectedThread,
  activeGameRooms,
  activeJoinedChannels,
  selectedPrivateChannelId,
  mediaObjectUrls,
  shellChromeState,
  selectedAuthorPubkey,
  previewableMediaAttachments,
  remoteObjectUrlRef,
  draftPreviewUrlRef,
  directMessageDraftPreviewUrlRef,
  mediaFetchAttemptRef,
  visibleRefreshInFlightRef,
  loadTopics,
  refreshVisibleShellData,
  setNotificationStatus,
  setLocalProfile,
  setProfileDraft,
  setKnownAuthorsByPubkey,
  setProfileTimeline,
  setProfileTimelineNextCursor,
  setProfileError,
  setProfilePanelState,
  setSocialConnections,
  setSocialConnectionsPanelState,
  setSelectedAuthor,
  setSelectedAuthorTimeline,
  setSelectedAuthorTimelineNextCursor,
  setAuthorError,
  setDirectMessages,
  setDirectMessageTimelineByPeer,
  setDirectMessageTimelineNextCursorByPeer,
  setDirectMessageStatusByPeer,
  setDirectMessageError,
  setNotifications,
  setNotificationPanelState,
  setNotificationAutoReadError,
  setGameDrafts,
  setSelectedChannelIdByTopic,
  setComposeChannelByTopic,
  setTimelineScopeByTopic,
  setMediaObjectUrls,
}: UseDesktopShellDataEffectsArgs) {
  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (
        disposed ||
        visibleRefreshInFlightRef.current ||
        (typeof document !== 'undefined' && document.visibilityState === 'hidden')
      ) {
        return;
      }
      visibleRefreshInFlightRef.current = true;
      try {
        await refreshVisibleShellData(activeTopic, selectedThread, 'buffer');
      } finally {
        visibleRefreshInFlightRef.current = false;
      }
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);
    const handleFocus = () => {
      void refresh();
    };
    const handleVisibility = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
        void refresh();
      }
    };
    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      disposed = true;
      visibleRefreshInFlightRef.current = false;
      window.clearInterval(intervalId);
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [activeTopic, refreshVisibleShellData, selectedThread, visibleRefreshInFlightRef]);

  useEffect(() => {
    let disposed = false;

    const refreshStatus = async () => {
      if (
        disposed ||
        (typeof document !== 'undefined' && document.visibilityState === 'hidden')
      ) {
        return;
      }
      try {
        const status = await api.getNotificationStatus();
        if (!disposed) {
          setNotificationStatus(status);
        }
      } catch {
        // best effort badge refresh
      }
    };

    void refreshStatus();
    const intervalId = window.setInterval(() => {
      void refreshStatus();
    }, STATUS_REFRESH_INTERVAL_MS);
    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [api, setNotificationStatus]);

  useEffect(() => {
    let disposed = false;
    void (async () => {
      try {
        const profile = await api.getMyProfile();
        if (disposed) {
          return;
        }
        setLocalProfile(profile);
        if (!storeApi.getState().profileDirty) {
          setProfileDraft(profileInputFromProfile(profile));
        }
      } catch {
        // best effort background bootstrap
      }
    })();
    return () => {
      disposed = true;
    };
  }, [api, setLocalProfile, setProfileDraft, storeApi]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'live') {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [activeTopic, loadTopics, selectedThread, shellChromeState.activePrimarySection, trackedTopics]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'game') {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [activeTopic, loadTopics, selectedThread, shellChromeState.activePrimarySection, trackedTopics]);

  useEffect(() => {
    if (
      shellChromeState.activePrimarySection !== 'timeline' ||
      shellChromeState.timelineView !== 'bookmarks'
    ) {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [
    activeTopic,
    loadTopics,
    selectedThread,
    shellChromeState.activePrimarySection,
    shellChromeState.timelineView,
    trackedTopics,
  ]);

  useEffect(() => {
    if (!shellChromeState.settingsOpen) {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [
    activeTopic,
    loadTopics,
    selectedThread,
    shellChromeState.activeSettingsSection,
    shellChromeState.settingsOpen,
    trackedTopics,
  ]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'profile') {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const profile = await api.getMyProfile();
        if (disposed) {
          return;
        }
        setLocalProfile(profile);
        if (!storeApi.getState().profileDirty) {
          setProfileDraft(profileInputFromProfile(profile));
        }
        const [timeline, following, followed, muted] = await Promise.all([
          api.listProfileTimeline(profile.pubkey, null, VISIBLE_TIMELINE_LIMIT),
          api.listSocialConnections('following'),
          api.listSocialConnections('followed'),
          api.listSocialConnections('muted'),
        ]);
        if (disposed) {
          return;
        }
        startTransition(() => {
          setProfileTimeline(timeline.items);
          setProfileTimelineNextCursor(timeline.next_cursor ?? null);
          setProfilePanelState({ status: 'ready', error: null });
          setProfileError(null);
          setSocialConnections({
            following,
            followed,
            muted,
          });
          setKnownAuthorsByPubkey((current) =>
            mergeKnownAuthors(current, [...following, ...followed, ...muted])
          );
          setSocialConnectionsPanelState({ status: 'ready', error: null });
        });
      } catch (error) {
        if (!disposed) {
          const message = messageFromError(
            error,
            translate('common:errors.failedToLoadProfile')
          );
          setProfileError(message);
          setProfilePanelState({ status: 'error', error: message });
        }
      }
    })();
    return () => {
      disposed = true;
    };
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
    shellChromeState.activePrimarySection,
    storeApi,
    translate,
  ]);

  useEffect(() => {
    if (!selectedAuthorPubkey) {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const [author, timeline] = await Promise.all([
          api.getAuthorSocialView(selectedAuthorPubkey),
          api.listProfileTimeline(selectedAuthorPubkey, null, VISIBLE_TIMELINE_LIMIT),
        ]);
        if (disposed) {
          return;
        }
        startTransition(() => {
          setSelectedAuthor(author);
          setSelectedAuthorTimeline(timeline.items);
          setSelectedAuthorTimelineNextCursor(timeline.next_cursor ?? null);
          setAuthorError(null);
          if (author) {
            setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, [author]));
          }
        });
      } catch (error) {
        if (!disposed) {
          setAuthorError(
            messageFromError(error, translate('common:errors.failedToLoadAuthor'))
          );
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, [
    api,
    selectedAuthorPubkey,
    setAuthorError,
    setKnownAuthorsByPubkey,
    setSelectedAuthor,
    setSelectedAuthorTimeline,
    setSelectedAuthorTimelineNextCursor,
    translate,
  ]);

  useEffect(() => {
    if (
      shellChromeState.activePrimarySection !== 'messages' &&
      !storeApi.getState().directMessagePaneOpen
    ) {
      return;
    }
    let disposed = false;
    const refresh = async () => {
      if (
        disposed ||
        (typeof document !== 'undefined' && document.visibilityState === 'hidden')
      ) {
        return;
      }
      try {
        const directMessages = await api.listDirectMessages();
        if (disposed) {
          return;
        }
        setDirectMessages(directMessages);
        setKnownAuthorsByPubkey((current) =>
          mergeKnownAuthors(current, directMessages.map(authorViewFromDirectMessageConversation))
        );
        const selectedPeerPubkey = storeApi.getState().selectedDirectMessagePeerPubkey;
        if (!selectedPeerPubkey) {
          setDirectMessageError(null);
          return;
        }
        const [timelineResult, statusResult] = await Promise.allSettled([
          api.listDirectMessageMessages(selectedPeerPubkey, null, VISIBLE_TIMELINE_LIMIT),
          api.getDirectMessageStatus(selectedPeerPubkey),
        ]);
        if (disposed) {
          return;
        }
        startTransition(() => {
          if (timelineResult.status === 'fulfilled') {
            setDirectMessageTimelineByPeer((current) => ({
              ...current,
              [selectedPeerPubkey]: timelineResult.value.items,
            }));
            setDirectMessageTimelineNextCursorByPeer((current) => ({
              ...current,
              [selectedPeerPubkey]: timelineResult.value.next_cursor ?? null,
            }));
          }
          if (statusResult.status === 'fulfilled') {
            setDirectMessageStatusByPeer((current) => ({
              ...current,
              [selectedPeerPubkey]: statusResult.value,
            }));
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
                  'failed to load direct messages'
                )
          );
        });
      } catch (error) {
        if (!disposed) {
          setDirectMessageError(messageFromError(error, 'failed to load direct messages'));
        }
      }
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);
    const handleFocus = () => {
      void refresh();
    };
    const handleVisibility = () => {
      if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
        void refresh();
      }
    };
    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleVisibility);
    return () => {
      disposed = true;
      window.clearInterval(intervalId);
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [
    api,
    setDirectMessageError,
    setDirectMessages,
    setDirectMessageStatusByPeer,
    setDirectMessageTimelineByPeer,
    setDirectMessageTimelineNextCursorByPeer,
    setKnownAuthorsByPubkey,
    shellChromeState.activePrimarySection,
    storeApi,
  ]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'notifications') {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const [status, notificationItems] = await Promise.all([
          api.getNotificationStatus(),
          api.listNotifications(),
        ]);
        if (disposed) {
          return;
        }
        let nextNotifications = notificationItems;
        let nextStatus = status;
        if (notificationItems.some((notification) => !notification.read_at)) {
          try {
            nextStatus = await api.markAllNotificationsRead();
            const readAt = Date.now();
            nextNotifications = notificationItems.map((notification) =>
              notification.read_at ? notification : { ...notification, read_at: readAt }
            );
            if (!disposed) {
              setNotificationAutoReadError(null);
            }
          } catch (notificationReadError) {
            if (!disposed) {
              setNotificationAutoReadError(
                messageFromError(
                  notificationReadError,
                  translate('shell:notifications.errors.failedAutoRead')
                )
              );
            }
          }
        }
        if (disposed) {
          return;
        }
        startTransition(() => {
          setNotificationStatus(nextStatus);
          setNotifications(nextNotifications);
          setNotificationPanelState({ status: 'ready', error: null });
        });
      } catch (error) {
        if (!disposed) {
          setNotificationPanelState({
            status: 'error',
            error: messageFromError(error, translate('shell:notifications.errors.failedToLoad')),
          });
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, [
    api,
    setNotificationAutoReadError,
    setNotificationPanelState,
    setNotifications,
    setNotificationStatus,
    shellChromeState.activePrimarySection,
    translate,
  ]);

  useEffect(() => {
    const remoteObjectUrls = remoteObjectUrlRef.current;
    const draftPreviewUrls = draftPreviewUrlRef.current;
    const directMessageDraftPreviewUrls = directMessageDraftPreviewUrlRef.current;

    return () => {
      for (const url of remoteObjectUrls.values()) {
        URL.revokeObjectURL(url);
      }
      remoteObjectUrls.clear();
      for (const url of draftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      draftPreviewUrls.clear();
      for (const url of directMessageDraftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      directMessageDraftPreviewUrls.clear();
    };
  }, [directMessageDraftPreviewUrlRef, draftPreviewUrlRef, remoteObjectUrlRef]);

  useEffect(() => {
    setGameDrafts((current) => {
      let changed = false;
      const next = { ...current };
      for (const room of activeGameRooms) {
        if (!next[room.room_id]) {
          next[room.room_id] = createGameEditorDraft(room);
          changed = true;
        }
      }
      return changed ? next : current;
    });
  }, [activeGameRooms, setGameDrafts]);

  useEffect(() => {
    if (!selectedPrivateChannelId) {
      return;
    }
    const selectedStillJoined = activeJoinedChannels.some(
      (channel) => channel.channel_id === selectedPrivateChannelId
    );
    if (selectedStillJoined) {
      return;
    }
    setSelectedChannelIdByTopic((current) => ({
      ...current,
      [activeTopic]: null,
    }));
    setComposeChannelByTopic((current) =>
      current[activeTopic]?.kind === 'private_channel' &&
      current[activeTopic].channel_id === selectedPrivateChannelId
        ? {
            ...current,
            [activeTopic]: PUBLIC_CHANNEL_REF,
          }
        : current
    );
    setTimelineScopeByTopic((current) =>
      current[activeTopic]?.kind === 'channel' &&
      current[activeTopic].channel_id === selectedPrivateChannelId
        ? {
            ...current,
            [activeTopic]: PUBLIC_TIMELINE_SCOPE,
          }
        : current
    );
  }, [
    activeJoinedChannels,
    activeTopic,
    selectedPrivateChannelId,
    setComposeChannelByTopic,
    setSelectedChannelIdByTopic,
    setTimelineScopeByTopic,
  ]);

  useEffect(() => {
    let disposed = false;

    for (const attachment of previewableMediaAttachments) {
      if (typeof mediaObjectUrls[attachment.hash] === 'string') {
        continue;
      }

      const nextAttempt = (mediaFetchAttemptRef.current.get(attachment.hash) ?? 0) + 1;
      mediaFetchAttemptRef.current.set(attachment.hash, nextAttempt);
      logMediaDebug('info', 'remote media fetch start', {
        attempt: nextAttempt,
        hash: attachment.hash,
        mime: attachment.mime,
        role: attachment.role,
        status: attachment.status,
      });

      void api
        .getBlobMediaPayload(attachment.hash, attachment.mime)
        .then((payload) => {
          const nextUrl = payload ? createObjectUrlFromPayload(payload) : null;
          if (disposed) {
            if (nextUrl) {
              URL.revokeObjectURL(nextUrl);
            }
            return;
          }
          if (!nextUrl) {
            logMediaDebug('warn', 'remote media fetch missing', {
              attempt: nextAttempt,
              hash: attachment.hash,
              mime: attachment.mime,
              role: attachment.role,
              status: attachment.status,
            });
            return;
          }

          logMediaDebug('info', 'remote media fetch hit', {
            attempt: nextAttempt,
            bytes_base64_length: payload?.bytes_base64.length ?? 0,
            hash: attachment.hash,
            mime: attachment.mime,
            object_url: nextUrl,
            role: attachment.role,
            status: attachment.status,
          });

          setMediaObjectUrls((current) => {
            if (current[attachment.hash] !== undefined) {
              URL.revokeObjectURL(nextUrl);
              return current;
            }
            remoteObjectUrlRef.current.set(attachment.hash, nextUrl);
            return {
              ...current,
              [attachment.hash]: nextUrl,
            };
          });
        })
        .catch((fetchError: unknown) => {
          if (disposed) {
            return;
          }
          logMediaDebug('warn', 'remote media fetch error', {
            attempt: nextAttempt,
            error: fetchError instanceof Error ? fetchError.message : 'unknown error',
            hash: attachment.hash,
            mime: attachment.mime,
            role: attachment.role,
            status: attachment.status,
          });
        });
    }

    return () => {
      disposed = true;
    };
  }, [
    api,
    mediaFetchAttemptRef,
    mediaObjectUrls,
    previewableMediaAttachments,
    remoteObjectUrlRef,
    setMediaObjectUrls,
  ]);
}
