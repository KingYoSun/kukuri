import { describe, it, expect, beforeAll, afterAll, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SyncStatusIndicator } from '@/components/SyncStatusIndicator';
import { useSyncManager } from '@/hooks/useSyncManager';
import type { PendingActionSummary } from '@/hooks/useSyncManager';
import type { SyncQueueItem, OfflineRetryMetrics } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/hooks/useSyncManager');
vi.mock('@/hooks/usePosts', () => ({
  useDeletePost: () => ({
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
    isPending: false,
    manualRetryDelete: vi.fn(),
  }),
}));
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
  },
}));

type CategoryKey = 'topic' | 'post' | 'follow' | 'dm';

const baseRetryMetrics: OfflineRetryMetrics = {
  totalSuccess: 1,
  totalFailure: 0,
  consecutiveFailure: 0,
  lastOutcome: 'success',
  lastJobId: 'nightly',
  lastJobReason: 'nightly',
  lastTrigger: 'manual',
  lastUserPubkey: 'npub1',
  lastRetryCount: 1,
  lastMaxRetries: 3,
  lastBackoffMs: 500,
  lastDurationMs: 900,
  lastSuccessCount: 1,
  lastFailureCount: 0,
  lastTimestampMs: Date.now(),
  lastSuccessMs: Date.now(),
  lastFailureMs: null,
};

const createPendingSummary = (category: CategoryKey): PendingActionSummary => ({
  total: 2,
  categories: [
    {
      category,
      count: 2,
      actionTypes: [`${category}_action`],
      samples: [
        { localId: `${category}-1`, actionType: `${category}_action`, targetId: `${category}-target-1` },
      ],
    },
  ],
});

const queueItemForCategory = (category: CategoryKey): SyncQueueItem => ({
  id: Math.floor(Math.random() * 1000) + 1,
  action_type:
    category === 'topic'
      ? OfflineActionType.TOPIC_CREATE
      : category === 'post'
        ? OfflineActionType.CREATE_POST
        : category === 'follow'
          ? OfflineActionType.FOLLOW
          : 'send_direct_message',
  status: 'pending',
  retry_count: 0,
  max_retries: 3,
  created_at: Date.now(),
  updated_at: Date.now(),
  payload: {
    cacheType: `${category}_cache`,
    requestedAt: new Date().toISOString(),
    requestedBy: 'nightly',
    source: 'sync_status_indicator',
  },
});

const getCategoryFixture = (category: CategoryKey) => ({
  pendingSummary: createPendingSummary(category),
  queueItems: [queueItemForCategory(category)],
  retryMetrics: {
    ...baseRetryMetrics,
    lastJobReason: `${category}_nightly`,
  },
});

const OFFLINE_CATEGORY = (process.env.OFFLINE_SYNC_CATEGORY as CategoryKey | undefined) ?? 'topic';
if (!['topic', 'post', 'follow', 'dm'].includes(OFFLINE_CATEGORY)) {
  throw new Error(`Unknown OFFLINE_SYNC_CATEGORY: ${OFFLINE_CATEGORY}`);
}

const defaultManagerState = {
  syncStatus: {
    isSyncing: false,
    progress: 0,
    totalItems: 0,
    syncedItems: 0,
    conflicts: [],
  },
  triggerManualSync: vi.fn(),
  resolveConflict: vi.fn(),
  updateProgress: vi.fn(),
  pendingActionsCount: 0,
  isOnline: true,
  cacheStatus: null,
  cacheStatusError: null,
  isCacheStatusLoading: false,
  refreshCacheStatus: vi.fn(),
  queueItems: [],
  queueItemsError: null,
  isQueueItemsLoading: false,
  refreshQueueItems: vi.fn(),
  lastQueuedItemId: null,
  queueingType: null,
  enqueueSyncRequest: vi.fn(),
  retryMetrics: baseRetryMetrics,
  retryMetricsError: null,
  isRetryMetricsLoading: false,
  refreshRetryMetrics: vi.fn(),
  scheduledRetry: null,
  showConflictDialog: false,
  setShowConflictDialog: vi.fn(),
  pendingActionSummary: {
    total: 0,
    categories: [],
  },
};

describe(`Offline Sync Telemetry (${OFFLINE_CATEGORY})`, () => {
  beforeAll(() => {
    errorHandler.setTestEnvironment('development');
  });

  afterAll(() => {
    errorHandler.setTestEnvironment(null);
  });

  it('SyncStatus telemetry is emitted for the active category', () => {
    const fixture = getCategoryFixture(OFFLINE_CATEGORY);
    const infoSpy = vi.spyOn(errorHandler, 'info');
    vi.mocked(useSyncManager).mockReturnValue({
      ...defaultManagerState,
      pendingActionsCount: fixture.pendingSummary.total,
      pendingActionSummary: fixture.pendingSummary,
      queueItems: fixture.queueItems,
      retryMetrics: fixture.retryMetrics,
    });

    render(<SyncStatusIndicator />);

    expect(infoSpy).toHaveBeenCalledWith(
      'SyncStatus.pending_actions_snapshot',
      'SyncStatusIndicator.telemetry',
      expect.objectContaining({ total: fixture.pendingSummary.total }),
    );
    expect(infoSpy).toHaveBeenCalledWith(
      'SyncStatus.queue_snapshot',
      'SyncStatusIndicator.telemetry',
      expect.objectContaining({ total: fixture.queueItems.length }),
    );
    expect(infoSpy).toHaveBeenCalledWith(
      'SyncStatus.retry_metrics_snapshot',
      'SyncStatusIndicator.telemetry',
      expect.objectContaining({ totalSuccess: fixture.retryMetrics.totalSuccess }),
    );
    expect(screen.queryByText('オフライン')).not.toBeInTheDocument();
    infoSpy.mockRestore();
  });
});
