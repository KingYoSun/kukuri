import {
  startTransition,
  useCallback,
  useEffect,
  useRef,
  type MutableRefObject,
} from 'react';

import { reconcileCommunityIndexNodePreference } from '@/lib/api/communityIndex';
import type {
  AttachmentView,
  CommunityNodeNodeStatus,
  DesktopApi,
  GameRoomView,
  SyncStatus,
} from '@/lib/api';
import type { ShellChromeProjection } from '@/components/shell/types';

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
  useDesktopShellFieldSetter,
  useDesktopShellStore,
  type DesktopShellState,
  type DesktopShellStateValue,
  type DesktopShellStoreApi,
} from '@/shell/store';
import { activeWorkspaceScope } from '@/shell/slices/workspace';
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
  activeGameRooms: GameRoomView[];
  activeJoinedChannels: DesktopShellState['joinedChannelsByTopic'][string];
  selectedPrivateChannelId: string | null;
  mediaObjectUrls: DesktopShellState['mediaObjectUrls'];
  shellChromeState: ShellChromeProjection;
  selectedAuthorPubkey: string | null;
  previewableMediaAttachments: AttachmentView[];
  /// #858: 表示設定 OFF の間にゲート対象となる成人向け添付 hash。
  gatedAdultMediaHashes: string[];
  remoteObjectUrlRef: MutableRefObject<Map<string, string>>;
  draftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  directMessageDraftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  mediaFetchAttemptRef: MutableRefObject<Map<string, number>>;
  visibleRefreshInFlightRef: MutableRefObject<boolean>;
  /// 表示中の Column id 列(DesktopShellColumnWorkspace の IntersectionObserver 由来)。
  visibleColumnIdsRef?: MutableRefObject<string[]>;
  loadTopics: (topics: string[], activeTopic: string, currentThread: string | null) => Promise<void>;
  // section 取得ロジックの SSoT は data/loaders/useDesktopShellSectionLoaders.ts。
  // この hook は「いつ読むか」(section 遷移・interval)だけを持ち、
  // 「何をどう読むか」は loader を呼ぶ。
  loadProfileSection: () => Promise<void>;
  loadAuthorSection: (pubkey: string) => Promise<void>;
  loadMessagesSection: () => Promise<void>;
  loadNotificationsSection: (options?: { markAsRead?: boolean }) => Promise<void>;
  loadCommunityIndexCapability: (
    refreshedStatuses?: readonly CommunityNodeNodeStatus[]
  ) => Promise<void>;
  refreshVisibleShellData: (
    topic: string,
    currentThread: string | null,
    mode?: 'apply' | 'buffer',
    scopeChannelId?: string | null
  ) => Promise<void>;
  refreshConnectivityStatus: () => Promise<CommunityNodeNodeStatus[] | null>;
  setNotificationStatus: Setter<'notificationStatus'>;
  setCommunityNodeStatuses: Setter<'communityNodeStatuses'>;
  setSyncStatus: Setter<'syncStatus'>;
  setLocalProfile: Setter<'localProfile'>;
  setProfileDraft: Setter<'profileDraft'>;
  setNotifications: Setter<'notifications'>;
  setGameDrafts: Setter<'gameDrafts'>;
  setSelectedChannelIdByTopic: (
    value:
      | Record<string, string | null>
      | ((current: Record<string, string | null>) => Record<string, string | null>)
  ) => void;
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
  gatedAdultMediaHashes,
  remoteObjectUrlRef,
  draftPreviewUrlRef,
  directMessageDraftPreviewUrlRef,
  mediaFetchAttemptRef,
  visibleRefreshInFlightRef,
  visibleColumnIdsRef,
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
  const mediaFetchInputRef = useRef(new Map<string, AttachmentView>());
  const setAdultContentEnabled = useDesktopShellFieldSetter('adultContentEnabled');

  // #858: 成人向け表現の表示設定(canonical は Rust 側ローカル JSON)を起動時に mirror する。
  useEffect(() => {
    let disposed = false;
    void api
      .getContentDisplaySettings()
      .then((settings) => {
        if (!disposed) {
          setAdultContentEnabled(settings.adult_content_enabled);
        }
      })
      .catch(() => {
        // 読めない場合は既定 OFF のまま(fail-closed)。
      });
    return () => {
      disposed = true;
    };
  }, [api, setAdultContentEnabled]);

  // #858: 表示設定 OFF の間、ゲート対象 hash の表示済み object URL を破棄し、
  // 取得試行の記録も消して以後の取得を停止する(ON へ戻せば再取得される)。
  useEffect(() => {
    if (gatedAdultMediaHashes.length === 0) {
      return;
    }
    for (const hash of gatedAdultMediaHashes) {
      const url = remoteObjectUrlRef.current.get(hash);
      if (url) {
        URL.revokeObjectURL(url);
        remoteObjectUrlRef.current.delete(hash);
      }
      mediaFetchInputRef.current.delete(hash);
      mediaFetchAttemptRef.current.delete(hash);
    }
    setMediaObjectUrls((current) => {
      let changed = false;
      const next = { ...current };
      for (const hash of gatedAdultMediaHashes) {
        if (hash in next) {
          delete next[hash];
          changed = true;
        }
      }
      return changed ? next : current;
    });
  }, [gatedAdultMediaHashes, mediaFetchAttemptRef, remoteObjectUrlRef, setMediaObjectUrls]);
  // 非 active な Timeline Column が Bookmarks を表示しているか(bookmarks ロード gate 用、Issue #765)。
  const hasBookmarksTimelineColumn = useDesktopShellStore((state) =>
    state.workspaceState.columns.some((column) => column.timelineView === 'bookmarks')
  );
  const hasBackgroundNotificationsColumn = useDesktopShellStore((state) =>
    state.workspaceState.columns.some(
      (column) =>
        column.kind === 'notifications' && column.id !== state.workspaceState.activeColumnId
    )
  );
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
        // Issue #765: 表示中の背景 Timeline Column の scope も定期 refresh する。
        // active scope(選択 channel と public)は上で取得済みなので除外し、
        // 非表示 Column は取得しない(API 呼び出し数の上限 = 表示中 Column 数)。
        const currentState = storeApi.getState();
        const visibleIds = new Set(visibleColumnIdsRef?.current ?? []);
        const activeScope = activeWorkspaceScope(currentState.workspaceState);
        const activeSelectedChannelId =
          activeScope.topicId === activeTopic ? activeScope.channelId : null;
        const seenScopeKeys = new Set<string>([
          `${activeTopic}\u0000${activeSelectedChannelId ?? ''}`,
          `${activeTopic}\u0000`,
        ]);
        const backgroundScopes = currentState.workspaceState.columns.flatMap((column) => {
          if (column.kind !== 'timeline' || !column.scope) return [];
          if (!visibleIds.has(column.id)) return [];
          const key = `${column.scope.topicId}\u0000${column.scope.channelId ?? ''}`;
          if (seenScopeKeys.has(key)) return [];
          seenScopeKeys.add(key);
          return [column.scope];
        });
        for (const scope of backgroundScopes) {
          if (disposed) break;
          await refreshVisibleShellData(scope.topicId, null, 'buffer', scope.channelId);
        }
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
  }, [
    activeTopic,
    refreshVisibleShellData,
    selectedThread,
    storeApi,
    visibleColumnIdsRef,
    visibleRefreshInFlightRef,
  ]);

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
        const resolution = reconcileCommunityIndexNodePreference(state);
        if (
          resolution.selectedBaseUrl !== state.communityIndexNodeBaseUrl ||
          JSON.stringify(resolution.preference) !==
            JSON.stringify(state.communityIndexNodePreference)
        ) {
          state.patchState({
            communityIndexNodeBaseUrl: resolution.selectedBaseUrl,
            communityIndexNodePreference: resolution.preference,
          });
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
    // Bookmarks データは chrome projection(active Column)だけでなく、非 active な
    // Timeline Column が Bookmarks を表示している場合もロードする(Issue #765)。
    if (
      (shellChromeState.activePrimarySection !== 'timeline' ||
        shellChromeState.timelineView !== 'bookmarks') &&
      !hasBookmarksTimelineColumn
    ) {
      return;
    }
    void loadTopics(trackedTopics, activeTopic, selectedThread).catch(() => undefined);
  }, [
    activeTopic,
    hasBookmarksTimelineColumn,
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
    const active = shellChromeState.activePrimarySection === 'notifications';
    if (!active && !hasBackgroundNotificationsColumn) {
      return;
    }
    void loadNotificationsSection({ markAsRead: active }).catch(() => undefined);
  }, [
    hasBackgroundNotificationsColumn,
    loadNotificationsSection,
    shellChromeState.activePrimarySection,
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
    const currentHashes = new Set(previewableMediaAttachments.map((attachment) => attachment.hash));
    for (const hash of mediaFetchInputRef.current.keys()) {
      if (!currentHashes.has(hash)) {
        mediaFetchInputRef.current.delete(hash);
      }
    }

    for (const attachment of previewableMediaAttachments) {
      if (typeof mediaObjectUrls[attachment.hash] === 'string') {
        continue;
      }
      if (mediaFetchInputRef.current.get(attachment.hash) === attachment) {
        continue;
      }
      mediaFetchInputRef.current.set(attachment.hash, attachment);

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
            setMediaObjectUrls((current) =>
              typeof current[attachment.hash] === 'string' || current[attachment.hash] === null
                ? current
                : { ...current, [attachment.hash]: null }
            );
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
            if (typeof current[attachment.hash] === 'string') {
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
          setMediaObjectUrls((current) =>
            typeof current[attachment.hash] === 'string' || current[attachment.hash] === null
              ? current
              : { ...current, [attachment.hash]: null }
          );
        });
    }

    return () => {
      disposed = true;
    };
  }, [
    api,
    mediaFetchAttemptRef,
    mediaFetchInputRef,
    mediaObjectUrls,
    previewableMediaAttachments,
    remoteObjectUrlRef,
    setMediaObjectUrls,
  ]);
}
