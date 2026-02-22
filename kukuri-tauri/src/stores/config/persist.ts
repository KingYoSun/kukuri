import {
  createMapAwareStorage,
  createPartializer,
  type PersistOptions,
} from '../utils/persistHelpers';

export const persistKeys = {
  auth: 'auth-storage',
  drafts: 'kukuri-drafts',
  offline: 'offline-store',
  p2p: 'p2p-storage',
  topic: 'topic-storage',
  ui: 'ui-storage',
  privacy: 'privacy-settings',
  keyManagement: 'key-management-history',
  communityNode: 'community-node-settings',
} as const;

export const createAuthPersistConfig = <
  T extends { currentUser: unknown | null },
>(): PersistOptions<T> => ({
  name: persistKeys.auth,
  partialize: createPartializer<T, 'currentUser'>(['currentUser']),
});

export const createDraftPersistConfig = <T extends { drafts: unknown[] }>(): PersistOptions<T> => ({
  name: persistKeys.drafts,
  partialize: createPartializer<T, 'drafts'>(['drafts']),
});

export const createOfflinePersistConfig = <
  T extends {
    lastSyncedAt?: number;
    pendingActions: unknown[];
    syncQueue: unknown[];
  },
>(): PersistOptions<T> => ({
  name: persistKeys.offline,
  partialize: createPartializer<T, 'lastSyncedAt' | 'pendingActions' | 'syncQueue'>([
    'lastSyncedAt',
    'pendingActions',
    'syncQueue',
  ]),
  storage: createMapAwareStorage(),
});

export const createP2PPersistConfig = <
  T extends { initialized: boolean; nodeId: string | null; nodeAddr: string | null },
>(): PersistOptions<T> => ({
  name: persistKeys.p2p,
  partialize: createPartializer<T, 'initialized' | 'nodeId' | 'nodeAddr'>([
    'initialized',
    'nodeId',
    'nodeAddr',
  ]),
  storage: createMapAwareStorage(),
});

export const createTopicPersistConfig = <
  T extends {
    joinedTopics: string[];
    currentTopic: unknown | null;
    topicUnreadCounts: Map<string, number>;
    topicLastReadAt: Map<string, number>;
  },
>(): PersistOptions<T> => ({
  name: persistKeys.topic,
  partialize: createPartializer<
    T,
    'joinedTopics' | 'currentTopic' | 'topicUnreadCounts' | 'topicLastReadAt'
  >(['joinedTopics', 'currentTopic', 'topicUnreadCounts', 'topicLastReadAt']),
  storage: createMapAwareStorage(),
});

export const createUiPersistConfig = <
  T extends { theme: 'light' | 'dark' | 'system' },
>(): PersistOptions<T> => ({
  name: persistKeys.ui,
  partialize: createPartializer<T, 'theme'>(['theme']),
});

export const createKeyManagementPersistConfig = <
  T extends { history: unknown[]; lastExportedAt: number | null; lastImportedAt: number | null },
>(): PersistOptions<T> => ({
  name: persistKeys.keyManagement,
  partialize: createPartializer<T, 'history' | 'lastExportedAt' | 'lastImportedAt'>([
    'history',
    'lastExportedAt',
    'lastImportedAt',
  ]),
});

export const createCommunityNodePersistConfig = <
  T extends {
    enableAccessControl: boolean;
  },
>(): PersistOptions<T> => ({
  name: persistKeys.communityNode,
  partialize: createPartializer<T, 'enableAccessControl'>(['enableAccessControl']),
});
