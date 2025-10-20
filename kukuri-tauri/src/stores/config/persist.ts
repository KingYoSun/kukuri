import { createMapAwareStorage, type PersistOptions } from '../utils/persistHelpers';

export const persistKeys = {
  auth: 'auth-storage',
  drafts: 'kukuri-drafts',
  offline: 'offline-store',
  p2p: 'p2p-storage',
  topic: 'topic-storage',
} as const;

export const createAuthPersistConfig = <
  T extends { currentUser: unknown | null },
>(): PersistOptions<T> => ({
  name: persistKeys.auth,
  partialize: (state) => ({
    // privateKey / 認証状態はセキュアストレージで扱うため永続化しない
    currentUser: state.currentUser,
  }),
});

export const createDraftPersistConfig = <
  T extends { drafts: unknown[] },
>(): PersistOptions<T> => ({
  name: persistKeys.drafts,
  partialize: (state) => ({
    // currentDraftId はリロード時の混乱を避けるため永続化しない
    drafts: state.drafts,
  }),
});

export const createOfflinePersistConfig = <
  T extends {
    lastSyncedAt: number | undefined;
    pendingActions: unknown[];
    syncQueue: unknown[];
  },
>(): PersistOptions<T> => ({
  name: persistKeys.offline,
  partialize: (state) => ({
    lastSyncedAt: state.lastSyncedAt,
    pendingActions: state.pendingActions,
    syncQueue: state.syncQueue,
  }),
  storage: createMapAwareStorage(),
});

export const createP2PPersistConfig = <
  T extends { initialized: boolean; nodeId: string | null; nodeAddr: string | null },
>(): PersistOptions<T> => ({
  name: persistKeys.p2p,
  partialize: (state) => ({
    // Map を含む詳細状態は実時間情報のため永続化しない
    initialized: state.initialized,
    nodeId: state.nodeId,
    nodeAddr: state.nodeAddr,
  }),
  storage: createMapAwareStorage(),
});

export const createTopicPersistConfig = <
  T extends { joinedTopics: string[]; currentTopic: unknown | null },
>(): PersistOptions<T> => ({
  name: persistKeys.topic,
  partialize: (state) => ({
    // topics Map は起動時に同期し直すため、参加情報のみ保持
    joinedTopics: state.joinedTopics,
    currentTopic: state.currentTopic,
  }),
  storage: createMapAwareStorage(),
});
