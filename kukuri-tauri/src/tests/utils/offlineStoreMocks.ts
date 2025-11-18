import { vi } from 'vitest';

import type { OfflineAction } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';

const buildOfflineStoreState = () => ({
  pendingActions: [] as OfflineAction[],
  isOnline: true,
  isSyncing: false,
  lastSyncedAt: undefined as number | undefined,
  syncPendingActions: vi.fn().mockResolvedValue(undefined),
  clearPendingActions: vi.fn(),
  removePendingAction: vi.fn(),
  setSyncError: vi.fn(),
  clearSyncError: vi.fn(),
  refreshCacheMetadata: vi.fn().mockResolvedValue(undefined),
  updateLastSyncedAt: vi.fn(),
  saveOfflineAction: vi.fn().mockResolvedValue(undefined),
  loadPendingActions: vi.fn().mockResolvedValue(undefined),
  cleanupExpiredCache: vi.fn().mockResolvedValue(undefined),
  applyOptimisticUpdate: vi.fn().mockResolvedValue(''),
  confirmUpdate: vi.fn().mockResolvedValue(undefined),
  rollbackUpdate: vi.fn().mockResolvedValue(null),
});

export type OfflineStoreTestState = ReturnType<typeof buildOfflineStoreState>;

export const createOfflineStoreTestState = (
  overrides?: Partial<OfflineStoreTestState>,
): OfflineStoreTestState => ({
  ...buildOfflineStoreState(),
  ...overrides,
});

export const createOfflineAction = (overrides?: Partial<OfflineAction>): OfflineAction => ({
  id: overrides?.id ?? 1,
  localId: overrides?.localId ?? 'local_1',
  userPubkey: overrides?.userPubkey ?? 'user123',
  actionType: overrides?.actionType ?? OfflineActionType.CREATE_POST,
  actionData: overrides?.actionData ?? { content: 'Test post' },
  createdAt: overrides?.createdAt ?? new Date().toISOString(),
  isSynced: overrides?.isSynced ?? false,
});
