import React from 'react';
import { useSyncManager } from '@/hooks/useSyncManager';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { ConflictResolutionDialog } from '@/components/sync/ConflictResolutionDialog';
import {
  extractDocConflictDetails,
  formatBytesValue,
  toNumber,
  truncateMiddle,
} from '@/components/sync/conflictUtils';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  RefreshCw,
  AlertCircle,
  CheckCircle,
  Clock,
  Wifi,
  WifiOff,
  AlertTriangle,
  Database,
  History,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import type { CacheTypeStatus, SyncQueueItem } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { useDeletePost } from '@/hooks/usePosts';

type CacheMetadataSummary = {
  cacheType?: string;
  requestedAt?: string;
  requestedBy?: string;
  queueItemId?: number;
  source?: string;
};

type MetadataRow = {
  key: string;
  label: string;
  value: React.ReactNode;
};

type QueueStatusPresentation = {
  label: string;
  className: string;
};

type CacheDocSummary = {
  docVersion?: number;
  blobHash?: string;
  payloadBytes?: number;
};

function parseCacheMetadata(
  metadata?: Record<string, unknown> | null,
): CacheMetadataSummary | null {
  if (!metadata) {
    return null;
  }
  const requestedAt =
    typeof metadata.requestedAt === 'string' ? (metadata.requestedAt as string) : undefined;
  const requestedBy =
    typeof metadata.requestedBy === 'string' ? (metadata.requestedBy as string) : undefined;
  const queueItemId =
    typeof metadata.queueItemId === 'number' ? (metadata.queueItemId as number) : undefined;
  const source = typeof metadata.source === 'string' ? (metadata.source as string) : undefined;
  const cacheType =
    typeof metadata.cacheType === 'string' ? (metadata.cacheType as string) : undefined;

  if (!requestedAt && !requestedBy && !queueItemId && !source && !cacheType) {
    return null;
  }

  return {
    cacheType,
    requestedAt,
    requestedBy,
    queueItemId,
    source,
  };
}

function formatMetadataTimestamp(value?: string) {
  if (!value) {
    return null;
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return formatDistanceToNow(date, { addSuffix: true, locale: ja });
}

function metadataRowsFromSummary(summary: CacheMetadataSummary): MetadataRow[] {
  const rows: MetadataRow[] = [];

  if (summary.cacheType) {
    rows.push({
      key: 'cacheType',
      label: '対象キャッシュ',
      value: summary.cacheType,
    });
  }

  if (summary.requestedBy) {
    rows.push({
      key: 'requestedBy',
      label: '最終要求者',
      value: (
        <code className="rounded bg-muted/50 px-1 py-0.5 font-mono text-[11px]">
          {summary.requestedBy}
        </code>
      ),
    });
  }

  if (summary.requestedAt) {
    const formatted = formatMetadataTimestamp(summary.requestedAt) ?? summary.requestedAt;
    rows.push({
      key: 'requestedAt',
      label: '要求時刻',
      value: (
        <span title={summary.requestedAt}>
          {formatted}
          {formatted !== summary.requestedAt && (
            <span className="ml-1 text-muted-foreground/70">{summary.requestedAt}</span>
          )}
        </span>
      ),
    });
  }

  if (typeof summary.queueItemId === 'number') {
    rows.push({
      key: 'queueItemId',
      label: 'キュー ID',
      value: `#${summary.queueItemId}`,
    });
  }

  if (summary.source) {
    rows.push({
      key: 'source',
      label: '発行元',
      value: summary.source,
    });
  }

  return rows;
}

function getPayloadString(
  payload: Record<string, unknown> | undefined,
  key: string,
): string | undefined {
  if (!payload) {
    return undefined;
  }
  const value = payload[key];
  return typeof value === 'string' ? value : undefined;
}

function formatRequester(value: string) {
  if (value.length <= 16) {
    return value;
  }
  return `${value.slice(0, 10)}…${value.slice(-4)}`;
}

function getQueueStatusPresentation(status: string): QueueStatusPresentation {
  switch (status) {
    case 'pending':
      return {
        label: '待機中',
        className: 'bg-amber-100 text-amber-900 dark:bg-amber-900/40 dark:text-amber-200',
      };
    case 'processing':
      return {
        label: '処理中',
        className: 'bg-sky-100 text-sky-900 dark:bg-sky-900/40 dark:text-sky-200',
      };
    case 'completed':
      return {
        label: '完了',
        className: 'bg-emerald-100 text-emerald-900 dark:bg-emerald-900/40 dark:text-emerald-200',
      };
    case 'failed':
      return {
        label: '失敗',
        className: 'bg-rose-100 text-rose-900 dark:bg-rose-900/40 dark:text-rose-200',
      };
    default:
      return {
        label: status,
        className: 'bg-muted text-foreground',
      };
  }
}

function getCacheDocSummary(type: CacheTypeStatus): CacheDocSummary | null {
  const docVersion = toNumber(type.doc_version ?? type.docVersion);
  const payloadBytes = toNumber(type.payload_bytes ?? type.payloadBytes);
  const blobHashCandidate = type.blob_hash ?? type.blobHash;
  const blobHash = typeof blobHashCandidate === 'string' ? blobHashCandidate : undefined;

  if (typeof docVersion === 'undefined' && typeof payloadBytes === 'undefined' && !blobHash) {
    return null;
  }

  return {
    docVersion,
    payloadBytes,
    blobHash,
  };
}

function formatRetryTimestamp(value?: number | null) {
  if (!value) {
    return '記録なし';
  }
  const date = new Date(value);
  return formatDistanceToNow(date, { addSuffix: true, locale: ja });
}

function formatDuration(ms?: number | null) {
  if (!ms) {
    return '未計測';
  }
  if (ms < 1000) {
    return `${ms}ms`;
  }
  return `${(ms / 1000).toFixed(1)}s`;
}

function formatBackoff(ms?: number | null) {
  if (!ms) {
    return '未設定';
  }
  if (ms < 1000) {
    return `${ms}ms`;
  }
  return `${Math.round(ms / 1000)}s`;
}

export function SyncStatusIndicator() {
  const {
    syncStatus,
    triggerManualSync,
    resolveConflict,
    pendingActionsCount,
    isOnline,
    cacheStatus,
    cacheStatusError,
    isCacheStatusLoading,
    refreshCacheStatus,
    queueItems,
    queueItemsError,
    isQueueItemsLoading,
    refreshQueueItems,
    lastQueuedItemId,
    queueingType,
    enqueueSyncRequest,
    retryMetrics,
    retryMetricsError,
    isRetryMetricsLoading,
    refreshRetryMetrics,
    scheduledRetry,
    showConflictDialog,
    setShowConflictDialog,
  } = useSyncManager();

  const [focusedConflictIndex, setFocusedConflictIndex] = React.useState(0);
  const [queueFilter, setQueueFilter] = React.useState('');
  const [retryingItemId, setRetryingItemId] = React.useState<number | null>(null);
  const deletePostMutation = useDeletePost();

  React.useEffect(() => {
    if (focusedConflictIndex >= syncStatus.conflicts.length) {
      setFocusedConflictIndex(
        syncStatus.conflicts.length > 0 ? syncStatus.conflicts.length - 1 : 0,
      );
    }
  }, [focusedConflictIndex, syncStatus.conflicts.length]);

  React.useEffect(() => {
    if (showConflictDialog && syncStatus.conflicts.length === 0) {
      setShowConflictDialog(false);
    }
  }, [showConflictDialog, syncStatus.conflicts.length, setShowConflictDialog]);

  const handleOpenConflictDialog = React.useCallback(
    (index = 0) => {
      setFocusedConflictIndex(index);
      setShowConflictDialog(true);
    },
    [setShowConflictDialog],
  );

  const handleQueueRequest = async (cacheType: string) => {
    try {
      await enqueueSyncRequest(cacheType);
    } catch {
      // エラーは useSyncManager 内で通知済み
    }
  };

  const handleDeleteRetry = React.useCallback(
    async (item: SyncQueueItem) => {
      const postId =
        getPayloadString(item.payload, 'postId') || getPayloadString(item.payload, 'entityId');
      if (!postId) {
        toast.error('削除対象の投稿IDを取得できませんでした');
        return;
      }
      const topicId = getPayloadString(item.payload, 'topicId');
      const authorPubkey =
        getPayloadString(item.payload, 'authorPubkey') ||
        getPayloadString(item.payload, 'userPubkey');

      try {
        setRetryingItemId(item.id);
        await deletePostMutation.manualRetryDelete({
          postId,
          topicId,
          authorPubkey,
        });
        toast.success('削除の再送を開始しました');
        await refreshQueueItems();
      } catch (error) {
        errorHandler.log('SyncQueue.post_delete_retry_failed', error, {
          context: 'SyncStatusIndicator.manualRetryDelete',
          metadata: {
            queueItemId: item.id,
            postId,
          },
        });
        toast.error('削除の再送に失敗しました');
      } finally {
        setRetryingItemId(null);
      }
    },
    [deletePostMutation, refreshQueueItems],
  );

  const normalizedQueueFilter = queueFilter.trim().toLowerCase();
  const filteredQueueItems = React.useMemo(() => {
    if (!normalizedQueueFilter) {
      return queueItems;
    }

    return queueItems.filter((item) => {
      const cacheType = getPayloadString(item.payload, 'cacheType')?.toLowerCase() ?? '';
      const idMatch = item.id.toString().includes(normalizedQueueFilter);
      const actionMatch = item.action_type?.toLowerCase().includes(normalizedQueueFilter) ?? false;
      return idMatch || actionMatch || cacheType.includes(normalizedQueueFilter);
    });
  }, [queueItems, normalizedQueueFilter]);

  const queueItemsToRender = React.useMemo(
    () => filteredQueueItems.slice(0, 20),
    [filteredQueueItems],
  );

  const getSyncStatusIcon = () => {
    if (!isOnline) {
      return <WifiOff className="h-4 w-4 text-muted-foreground" />;
    }

    if (syncStatus.isSyncing) {
      return <RefreshCw className="h-4 w-4 animate-spin text-blue-500" />;
    }

    if (syncStatus.conflicts.length > 0) {
      return <AlertTriangle className="h-4 w-4 text-yellow-500" />;
    }

    if (syncStatus.error) {
      return <AlertCircle className="h-4 w-4 text-red-500" />;
    }

    if (pendingActionsCount === 0) {
      return <CheckCircle className="h-4 w-4 text-green-500" />;
    }

    return <Clock className="h-4 w-4 text-muted-foreground" />;
  };

  const getSyncStatusText = () => {
    if (!isOnline) {
      return 'オフライン';
    }

    if (syncStatus.isSyncing) {
      return `同期中... (${syncStatus.syncedItems}/${syncStatus.totalItems})`;
    }

    if (syncStatus.conflicts.length > 0) {
      return `競合: ${syncStatus.conflicts.length}件`;
    }

    if (syncStatus.error) {
      return '同期エラー';
    }

    if (pendingActionsCount === 0) {
      return '同期済み';
    }

    return `未同期: ${pendingActionsCount}件`;
  };

  const formatCacheTypeLabel = (cacheType: string) => {
    switch (cacheType) {
      case 'sync_queue':
        return '同期キュー';
      case 'offline_actions':
        return 'オフラインアクション';
      default:
        return cacheType;
    }
  };

  const formatCacheLastSynced = (timestamp?: number | null) => {
    if (!timestamp) {
      return '記録なし';
    }
    return formatDistanceToNow(new Date(timestamp * 1000), {
      addSuffix: true,
      locale: ja,
    });
  };

  const docConflictCount = React.useMemo(
    () =>
      syncStatus.conflicts.filter(
        (conflict) =>
          extractDocConflictDetails(conflict.localAction) ||
          extractDocConflictDetails(conflict.remoteAction),
      ).length,
    [syncStatus.conflicts],
  );
  const firstConflict = syncStatus.conflicts[0] ?? null;

  return (
    <>
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="ghost" size="sm" className="gap-2" data-testid="sync-indicator">
            {getSyncStatusIcon()}
            <span className="text-sm">{getSyncStatusText()}</span>
            {pendingActionsCount > 0 && (
              <Badge variant="secondary" className="ml-1">
                {pendingActionsCount}
              </Badge>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-80">
          <div className="space-y-4">
            {syncStatus.conflicts.length > 0 && (
              <div
                className="rounded border border-amber-300 bg-amber-50 p-3 text-sm text-amber-900 dark:border-amber-500/60 dark:bg-amber-900/20 dark:text-amber-100"
                data-testid="sync-conflict-banner"
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="h-4 w-4" />
                    <span>
                      {docConflictCount > 0
                        ? `Doc/Blobの競合 ${docConflictCount}件`
                        : `競合 ${syncStatus.conflicts.length}件`}
                    </span>
                  </div>
                  {firstConflict && (
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 px-2 text-xs"
                      onClick={() => handleOpenConflictDialog(0)}
                    >
                      詳細を確認
                    </Button>
                  )}
                </div>
                {docConflictCount > 0 && (
                  <p className="mt-1 text-xs text-amber-900/80 dark:text-amber-100/80">
                    Doc/Blob の差分は競合ダイアログの「Doc/Blob」タブで比較できます。
                  </p>
                )}
              </div>
            )}

            {/* 同期状態 */}
            <div>
              <h4 className="font-medium mb-2 flex items-center gap-2">
                {isOnline ? (
                  <Wifi className="h-4 w-4 text-green-500" />
                ) : (
                  <WifiOff className="h-4 w-4 text-muted-foreground" />
                )}
                接続状態
              </h4>
              <p className="text-sm text-muted-foreground">
                {isOnline ? 'オンライン' : 'オフライン'}
              </p>
            </div>

            <div>
              <div className="mb-2 flex items-center justify-between">
                <h4 className="font-medium flex items-center gap-2">
                  <History className="h-4 w-4 text-primary" />
                  再送メトリクス
                </h4>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="再送メトリクスを更新"
                  onClick={() => {
                    void refreshRetryMetrics();
                  }}
                  disabled={isRetryMetricsLoading}
                >
                  <RefreshCw
                    className={`h-4 w-4 ${
                      isRetryMetricsLoading ? 'animate-spin text-muted-foreground' : ''
                    }`}
                  />
                </Button>
              </div>
              {retryMetricsError && (
                <p className="text-sm text-red-600 dark:text-red-400">{retryMetricsError}</p>
              )}
              {retryMetrics ? (
                <div className="space-y-2 text-sm">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span>成功 / 失敗</span>
                    <span>
                      <span className="font-semibold text-emerald-600 dark:text-emerald-300">
                        {retryMetrics.totalSuccess}
                      </span>
                      <span className="mx-1 text-muted-foreground">/</span>
                      <span className="font-semibold text-rose-600 dark:text-rose-300">
                        {retryMetrics.totalFailure}
                      </span>
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span>連続失敗</span>
                    <span>{retryMetrics.consecutiveFailure}</span>
                  </div>
                  {retryMetrics.lastOutcome && (
                    <div className="rounded border border-border/60 p-2 text-xs">
                      <div className="flex items-center justify-between">
                        <span className="font-medium">直近の再送</span>
                        <Badge
                          variant="outline"
                          className={cn(
                            'text-[10px]',
                            retryMetrics.lastOutcome === 'success'
                              ? 'border-emerald-500 text-emerald-600 dark:text-emerald-300'
                              : 'border-rose-500 text-rose-600 dark:text-rose-300',
                          )}
                        >
                          {retryMetrics.lastOutcome === 'success' ? '成功' : '失敗'}
                        </Badge>
                      </div>
                      <dl className="mt-1 space-y-1 text-muted-foreground">
                        <div className="flex items-center justify-between">
                          <dt>ID</dt>
                          <dd>{retryMetrics.lastJobId ?? '記録なし'}</dd>
                        </div>
                        <div className="flex items-center justify-between">
                          <dt>理由</dt>
                          <dd>{retryMetrics.lastJobReason ?? '未設定'}</dd>
                        </div>
                        <div className="flex items-center justify-between">
                          <dt>試行 / 上限</dt>
                          <dd>
                            {typeof retryMetrics.lastRetryCount === 'number' &&
                            typeof retryMetrics.lastMaxRetries === 'number'
                              ? `${retryMetrics.lastRetryCount}/${retryMetrics.lastMaxRetries}`
                              : '記録なし'}
                          </dd>
                        </div>
                        <div className="flex items-center justify-between">
                          <dt>Backoff</dt>
                          <dd>{formatBackoff(retryMetrics.lastBackoffMs)}</dd>
                        </div>
                        <div className="flex items-center justify-between">
                          <dt>実行時間</dt>
                          <dd>{formatDuration(retryMetrics.lastDurationMs)}</dd>
                        </div>
                        <div className="flex items-center justify-between">
                          <dt>計測</dt>
                          <dd>{formatRetryTimestamp(retryMetrics.lastTimestampMs)}</dd>
                        </div>
                      </dl>
                    </div>
                  )}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">
                  {isRetryMetricsLoading
                    ? '再送メトリクスを取得しています…'
                    : '再送メトリクスはまだ記録されていません'}
                </p>
              )}
              {scheduledRetry && (
                <p className="mt-2 text-xs text-muted-foreground">
                  次回 #{scheduledRetry.jobId} を{' '}
                  {formatMetadataTimestamp(scheduledRetry.nextRunAt) ?? scheduledRetry.nextRunAt}
                  に再送 （{scheduledRetry.retryCount + 1}/{scheduledRetry.maxRetries}）
                </p>
              )}
            </div>

            {/* 同期進捗 */}
            {syncStatus.isSyncing && (
              <div>
                <h4 className="font-medium mb-2">同期進捗</h4>
                <Progress value={syncStatus.progress} className="mb-2" />
                <p className="text-sm text-muted-foreground">
                  {syncStatus.syncedItems} / {syncStatus.totalItems} 件を同期中
                </p>
              </div>
            )}

            {/* 未同期アクション */}
            {pendingActionsCount > 0 && !syncStatus.isSyncing && (
              <div>
                <h4 className="font-medium mb-2">未同期アクション</h4>
                <p className="text-sm text-muted-foreground">
                  {pendingActionsCount}件のアクションが同期待ちです
                </p>
              </div>
            )}

            {/* 競合 */}
            {syncStatus.conflicts.length > 0 && (
              <div>
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4 text-yellow-500" />
                  競合検出
                </h4>
                <div className="space-y-2">
                  {syncStatus.conflicts.slice(0, 3).map((conflict, index) => (
                    <div
                      key={index}
                      className="text-sm p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded cursor-pointer hover:bg-yellow-100 dark:hover:bg-yellow-900/30"
                      onClick={() => handleOpenConflictDialog(index)}
                    >
                      <p className="font-medium">{conflict.localAction.actionType}</p>
                      <p className="text-xs text-muted-foreground">クリックして解決</p>
                    </div>
                  ))}
                  {syncStatus.conflicts.length > 3 && (
                    <p className="text-sm text-muted-foreground">
                      他 {syncStatus.conflicts.length - 3} 件の競合
                    </p>
                  )}
                </div>
              </div>
            )}

            {/* エラー */}
            {syncStatus.error && (
              <div>
                <h4 className="font-medium mb-2 flex items-center gap-2">
                  <AlertCircle className="h-4 w-4 text-red-500" />
                  同期エラー
                </h4>
                <p className="text-sm text-red-600 dark:text-red-400">{syncStatus.error}</p>
              </div>
            )}

            {/* 最終同期時刻 */}
            {syncStatus.lastSyncTime && (
              <div>
                <h4 className="font-medium mb-2">最終同期</h4>
                <p className="text-sm text-muted-foreground">
                  {formatDistanceToNow(syncStatus.lastSyncTime, {
                    addSuffix: true,
                    locale: ja,
                  })}
                </p>
              </div>
            )}

            {/* キャッシュ状態 */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-medium flex items-center gap-2">
                  <Database className="h-4 w-4 text-primary" />
                  キャッシュ状態
                </h4>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="キャッシュ情報を更新"
                  onClick={() => {
                    void refreshCacheStatus();
                  }}
                  disabled={isCacheStatusLoading}
                >
                  <RefreshCw
                    className={`h-4 w-4 ${isCacheStatusLoading ? 'animate-spin text-muted-foreground' : ''}`}
                  />
                </Button>
              </div>
              {cacheStatusError && (
                <p className="text-sm text-red-600 dark:text-red-400">{cacheStatusError}</p>
              )}
              {cacheStatus ? (
                <>
                  <p className="text-sm text-muted-foreground">
                    合計 {cacheStatus.total_items}件 / ステール {cacheStatus.stale_items}件
                  </p>
                  <div className="space-y-2 mt-2">
                    {(cacheStatus.cache_types ?? []).map((type) => {
                      const metadataSummary = parseCacheMetadata(type.metadata ?? null);
                      const docSummary = getCacheDocSummary(type);
                      return (
                        <div
                          key={type.cache_type}
                          className="rounded border border-border/60 p-2 text-sm"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div>
                              <p className="font-medium">{formatCacheTypeLabel(type.cache_type)}</p>
                              <p className="text-xs text-muted-foreground">
                                最終同期 {formatCacheLastSynced(type.last_synced_at)}
                              </p>
                            </div>
                            {type.is_stale && (
                              <Button
                                size="sm"
                                variant="outline"
                                className="h-7 px-2 text-xs"
                                onClick={() => {
                                  void handleQueueRequest(type.cache_type);
                                }}
                                disabled={!isOnline || queueingType === type.cache_type}
                              >
                                {queueingType === type.cache_type ? '追加中…' : '再送キュー'}
                              </Button>
                            )}
                          </div>
                          <p className="text-xs text-muted-foreground mt-1">
                            {type.item_count}件 / {type.is_stale ? '要再同期' : '最新'}
                          </p>
                          {metadataSummary &&
                            (() => {
                              const rows = metadataRowsFromSummary(metadataSummary);
                              if (rows.length === 0) {
                                return null;
                              }
                              return (
                                <dl
                                  className="mt-2 space-y-1 rounded-md bg-muted/40 p-2 text-xs text-muted-foreground"
                                  data-testid={`cache-metadata-${type.cache_type}`}
                                >
                                  {rows.map((row) => (
                                    <div
                                      className="flex items-start gap-2"
                                      key={`${type.cache_type}-${row.key}`}
                                    >
                                      <dt className="w-24 shrink-0 text-muted-foreground/80">
                                        {row.label}
                                      </dt>
                                      <dd className="flex-1 text-foreground">{row.value}</dd>
                                    </div>
                                  ))}
                                </dl>
                              );
                            })()}
                          {docSummary && (
                            <div
                              className="mt-2 rounded-md border border-amber-200 bg-amber-50 p-2 text-xs text-amber-900 dark:border-amber-500/60 dark:bg-amber-900/10 dark:text-amber-100"
                              data-testid={`cache-doc-${type.cache_type}`}
                            >
                              <p className="font-medium text-foreground">Doc/Blob キャッシュ</p>
                              <div className="mt-1 space-y-1 text-amber-900 dark:text-amber-50">
                                {typeof docSummary.docVersion !== 'undefined' && (
                                  <div className="flex items-center justify-between gap-2">
                                    <span>Doc Version</span>
                                    <code className="font-mono text-[11px]">
                                      {docSummary.docVersion}
                                    </code>
                                  </div>
                                )}
                                {docSummary.blobHash && (
                                  <div className="flex items-center justify-between gap-2">
                                    <span>Blob Hash</span>
                                    <code
                                      className="font-mono text-[11px]"
                                      title={docSummary.blobHash}
                                    >
                                      {truncateMiddle(docSummary.blobHash, 22)}
                                    </code>
                                  </div>
                                )}
                                {typeof docSummary.payloadBytes !== 'undefined' && (
                                  <div className="flex items-center justify-between gap-2">
                                    <span>Payload</span>
                                    <span>{formatBytesValue(docSummary.payloadBytes)}</span>
                                  </div>
                                )}
                              </div>
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </>
              ) : (
                <p className="text-sm text-muted-foreground">
                  {isCacheStatusLoading
                    ? 'キャッシュ情報を取得しています…'
                    : 'キャッシュ情報がまだありません'}
                </p>
              )}
            </div>

            {/* 再送キュー履歴 */}
            <div>
              <div className="mb-2 flex flex-col gap-2">
                <div className="flex items-center justify-between gap-2">
                  <h4 className="font-medium flex items-center gap-2">
                    <History className="h-4 w-4 text-primary" />
                    再送キュー履歴
                  </h4>
                  <div className="flex items-center gap-2">
                    {lastQueuedItemId && (
                      <span className="text-[11px] text-muted-foreground">
                        最新 #<code className="font-mono text-xs">{lastQueuedItemId}</code>
                      </span>
                    )}
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="再送キューを更新"
                      onClick={() => {
                        void refreshQueueItems();
                      }}
                      disabled={isQueueItemsLoading}
                    >
                      <RefreshCw
                        className={`h-4 w-4 ${
                          isQueueItemsLoading ? 'animate-spin text-muted-foreground' : ''
                        }`}
                      />
                    </Button>
                  </div>
                </div>
                <Input
                  value={queueFilter}
                  onChange={(event) => setQueueFilter(event.target.value)}
                  placeholder="Queue ID / cacheType を検索"
                  className="h-8 text-xs"
                  aria-label="再送キューをフィルタ"
                />
              </div>
              {queueItemsError && (
                <p className="text-sm text-red-600 dark:text-red-400">{queueItemsError}</p>
              )}
              {queueItemsToRender.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  {isQueueItemsLoading
                    ? '再送キューを取得しています…'
                    : '再送キューはまだ登録されていません'}
                </p>
              ) : (
                <div className="max-h-48 space-y-2 overflow-y-auto pr-1">
                  {queueItemsToRender.map((item) => {
                    const cacheType = getPayloadString(item.payload, 'cacheType');
                    const requestedBy = getPayloadString(item.payload, 'requestedBy');
                    const requestedAt = getPayloadString(item.payload, 'requestedAt');
                    const source = getPayloadString(item.payload, 'source');
                    const statusPresentation = getQueueStatusPresentation(item.status);
                    const updatedLabel = formatCacheLastSynced(item.updated_at);
                    const isHighlighted =
                      Boolean(lastQueuedItemId) &&
                      lastQueuedItemId === item.id &&
                      !normalizedQueueFilter;
                    const isDeleteAction = item.action_type === OfflineActionType.DELETE_POST;
                    const isRetryingDelete = retryingItemId === item.id;

                    return (
                      <div
                        key={item.id}
                        className={`rounded border p-2 text-xs transition ${
                          isHighlighted
                            ? 'border-primary bg-primary/5 shadow-sm'
                            : 'border-border/60'
                        }`}
                        data-testid={`queue-item-${item.id}`}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div>
                            <p className="flex items-center gap-2 text-sm font-medium">
                              <span>#{item.id}</span>
                              {cacheType && (
                                <code className="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">
                                  {cacheType}
                                </code>
                              )}
                            </p>
                            <p className="text-xs text-muted-foreground">
                              {item.action_type}・更新 {updatedLabel}
                            </p>
                          </div>
                          <Badge
                            className={`text-[10px] font-normal ${statusPresentation.className}`}
                          >
                            {statusPresentation.label}
                          </Badge>
                        </div>
                        <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                          <span>
                            再試行 {item.retry_count}/{item.max_retries}
                          </span>
                          {requestedBy && (
                            <span>
                              要求者{' '}
                              <code className="font-mono text-[11px]">
                                {formatRequester(requestedBy)}
                              </code>
                            </span>
                          )}
                          {source && <span>発行元 {source}</span>}
                          {requestedAt && (
                            <span title={requestedAt}>
                              要求 {formatMetadataTimestamp(requestedAt) ?? requestedAt}
                            </span>
                          )}
                        </div>
                        {item.error_message && (
                          <p className="mt-1 text-[11px] text-red-600 dark:text-red-400">
                            {item.error_message}
                          </p>
                        )}
                        {isDeleteAction && (
                          <div className="mt-2 flex justify-end">
                            <Button
                              size="sm"
                              variant="outline"
                              className="h-7 px-2 text-xs"
                              disabled={isRetryingDelete || deletePostMutation.isPending}
                              onClick={() => {
                                void handleDeleteRetry(item);
                              }}
                            >
                              <RefreshCw
                                className={cn(
                                  'mr-2 h-3.5 w-3.5',
                                  (isRetryingDelete || deletePostMutation.isPending) &&
                                    'animate-spin',
                                )}
                              />
                              削除を再送
                            </Button>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* 手動同期ボタン */}
            <Button
              onClick={() => {
                void triggerManualSync();
              }}
              disabled={!isOnline || syncStatus.isSyncing || pendingActionsCount === 0}
              className="w-full"
              size="sm"
            >
              <RefreshCw className="h-4 w-4 mr-2" />
              今すぐ同期
            </Button>
          </div>
        </PopoverContent>
      </Popover>

      {showConflictDialog && syncStatus.conflicts.length > 0 && (
        <ConflictResolutionDialog
          conflicts={syncStatus.conflicts}
          isOpen
          initialIndex={focusedConflictIndex}
          onClose={() => {
            setFocusedConflictIndex(0);
            setShowConflictDialog(false);
          }}
          onResolve={resolveConflict}
        />
      )}
    </>
  );
}
