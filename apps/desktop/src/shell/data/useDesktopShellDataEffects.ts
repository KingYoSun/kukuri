import {
  startTransition,
  useCallback,
  useEffect,
  type MutableRefObject,
} from 'react';

import { reconcileCommunityIndexNodeSelection } from '@/lib/api/communityIndex';
import type {
  AttachmentView,
  CommunityNodeNodeStatus,
  DesktopApi,
  SyncStatus,
} from '@/lib/api';

import {
  createObjectUrlFromPayload,
  logMediaDebug,
} from '@/shell/media';
import {
  PUBLIC_CHANNEL_REF,
  PUBLIC_TIMELINE_SCOPE,
  CONNECTIVITY_STATUS_FALLBACK_INTERVAL_MS,
  REFRESH_INTERVAL_MS,
  STATUS_REFRESH_INTERVAL_MS,
  type DesktopShellState,
  type DesktopShellStateValue,
  type DesktopShellStoreApi,
} from '@/shell/store';
import { setRecordEntry } from '@/shell/stateUpdates';
import {
  createGameEditorDraft,
  mergeCommunityNodeStatuses,
  profileInputFromProfile,
} from '@/shell/presentation';
import { useRuntimeEventBridge } from '@/shell/data/useRuntimeEventBridge';
import { isTauriRuntime } from '@/lib/releaseReadiness';

type Setter<K extends keyof DesktopShellState> = (
  value: DesktopShellStateValue<K>
) => void;

type UseDesktopShellDataEffectsArgs = {
  api: DesktopApi;
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
  // section 取得ロジックの SSoT は data/loaders/useDesktopShellSectionLoaders.ts。
  // この hook は「いつ読むか」(section 遷移・interval)だけを持ち、
  // 「何をどう読むか」は loader を呼ぶ。
  loadProfileSection: () => Promise<void>;
  loadAuthorSection: (pubkey: string) => Promise<void>;
  loadMessagesSection: () => Promise<void>;
  loadNotificationsSection: () => Promise<void>;
  loadCommunityIndexCapability: (
    refreshedStatuses?: readonly CommunityNodeNodeStatus[]
  ) => Promise<void>;
  refreshVisibleShellData: (
    topic: string,
    currentThread: string | null,
    mode?: 'apply' | 'buffer'
  ) => Promise<void>;
  refreshConnectivityStatus: () => Promise<CommunityNodeNodeStatus[] | null>;
  setNotificationStatus: Setter<'notificationStatus'>;
  setCommunityNodeStatuses: Setter<'communityNodeStatuses'>;
  setSyncStatus: Setter<'syncStatus'>;
  setLocalProfile: Setter<'localProfile'>;
  setProfileDraft: Setter<'profileDraft'>;
  setNotifications: Setter<'notifications'>;
  setGameDrafts: Setter<'gameDrafts'>;
  setSelectedChannelIdByTopic: Setter<'selectedChannelIdByTopic'>;
  setComposeChannelByTopic: Setter<'composeChannelByTopic'>;
  setTimelineScopeByTopic: Setter<'timelineScopeByTopic'>;
  setMediaObjectUrls: Setter<'mediaObjectUrls'>;
};

export function useDesktopShellDataEffects({
  api,
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
  loadProfileSection,
  loadAuthorSection,
  loadMessagesSection,
  loadNotificationsSection,
  loadCommunityIndexCapability,
  refreshVisibleShellData,
  refreshConnectivityStatus,
  setNotificationStatus,
  setCommunityNodeStatuses,
  setSyncStatus,
  setLocalProfile,
  setProfileDraft,
  setNotifications,
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

  const refreshNotificationStatus = useCallback(async () => {
    if (typeof document !== 'undefined' && document.visibilityState === 'hidden') {
      return;
    }
    try {
      const status = await api.getNotificationStatus();
      setNotificationStatus(status);
      if (
        status.unread_count > 0 &&
        shellChromeState.activePrimarySection !== 'notifications'
      ) {
        const notificationItems = await api.listNotifications();
        startTransition(() => {
          setNotifications(notificationItems);
        });
      }
    } catch {
      // best effort badge refresh
    }
  }, [api, setNotificationStatus, setNotifications, shellChromeState.activePrimarySection]);

  const applySyncStatusChange = useCallback(
    (
      syncStatus: SyncStatus | null,
      communityNodeStatuses: CommunityNodeNodeStatus[] | null
    ) => {
      startTransition(() => {
        if (syncStatus) {
          setSyncStatus(syncStatus);
        }
        if (communityNodeStatuses) {
          setCommunityNodeStatuses((current) =>
            mergeCommunityNodeStatuses(current, communityNodeStatuses)
          );
        }
      });
    },
    [setCommunityNodeStatuses, setSyncStatus]
  );

  useRuntimeEventBridge(refreshNotificationStatus, applySyncStatusChange);

  useEffect(() => {
    void refreshConnectivityStatus()
      .then((statuses) => loadCommunityIndexCapability(statuses ?? undefined))
      .catch(() => undefined);
    const intervalMs = isTauriRuntime()
      ? CONNECTIVITY_STATUS_FALLBACK_INTERVAL_MS
      : REFRESH_INTERVAL_MS;
    const intervalId = window.setInterval(() => {
      // 接続状態(認証・同意・通信エラー)が変わって適格ノードが入れ替わった場合に備え、
      // 定期更新の後は既存の構成情報記録で選択中の索引ノードを再調整する(#698)。
      void refreshConnectivityStatus().then((statuses) => {
        if (!statuses) return;
        const state = storeApi.getState();
        const next = reconcileCommunityIndexNodeSelection(state);
        if (next !== state.communityIndexNodeBaseUrl) {
          state.patchState({ communityIndexNodeBaseUrl: next });
        }
      });
    }, intervalMs);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [loadCommunityIndexCapability, refreshConnectivityStatus, storeApi]);

  useEffect(() => {
    void refreshNotificationStatus();
    const intervalId = window.setInterval(() => {
      void refreshNotificationStatus();
    }, STATUS_REFRESH_INTERVAL_MS);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [refreshNotificationStatus]);

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

  // 以下 4 つの section effect は live/game/bookmarks/settings と同じ委譲形:
  // トリガ判定だけを持ち、取得・state 反映は loaders/ の単一実装(SSoT)を呼ぶ。
  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'profile') {
      return;
    }
    void loadProfileSection().catch(() => undefined);
  }, [loadProfileSection, shellChromeState.activePrimarySection]);

  useEffect(() => {
    if (!selectedAuthorPubkey) {
      return;
    }
    void loadAuthorSection(selectedAuthorPubkey).catch(() => undefined);
  }, [loadAuthorSection, selectedAuthorPubkey]);

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
      await loadMessagesSection().catch(() => undefined);
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
  }, [loadMessagesSection, shellChromeState.activePrimarySection, storeApi]);

  useEffect(() => {
    if (shellChromeState.activePrimarySection !== 'notifications') {
      return;
    }
    void loadNotificationsSection().catch(() => undefined);
  }, [loadNotificationsSection, shellChromeState.activePrimarySection]);

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
    setSelectedChannelIdByTopic(setRecordEntry(activeTopic, null));
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
