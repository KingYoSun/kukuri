import React from 'react';
import { useSyncManager } from '@/hooks/useSyncManager';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
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
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import type { SyncConflict } from '@/lib/sync/syncEngine';

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
    enqueueSyncRequest,
  } = useSyncManager();

  const [selectedConflict, setSelectedConflict] = React.useState<SyncConflict | null>(null);
  const [showConflictDialog, setShowConflictDialog] = React.useState(false);
  const [queueingType, setQueueingType] = React.useState<string | null>(null);

  const handleConflictResolution = (resolution: 'local' | 'remote' | 'merge') => {
    if (selectedConflict) {
      resolveConflict(selectedConflict, resolution);
      setShowConflictDialog(false);
      setSelectedConflict(null);
    }
  };

  const handleQueueRequest = async (cacheType: string) => {
    setQueueingType(cacheType);
    try {
      await enqueueSyncRequest(cacheType);
    } catch {
      // エラーは useSyncManager 内で通知済み
    } finally {
      setQueueingType((current) => (current === cacheType ? null : current));
    }
  };

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
                      onClick={() => {
                        setSelectedConflict(conflict);
                        setShowConflictDialog(true);
                      }}
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
                    {cacheStatus.cache_types.map((type) => {
                      const metadataSummary = parseCacheMetadata(type.metadata ?? null);
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

            {/* 手動同期ボタン */}
            <Button
              onClick={triggerManualSync}
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

      {/* 競合解決ダイアログ */}
      <AlertDialog open={showConflictDialog} onOpenChange={setShowConflictDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>競合の解決</AlertDialogTitle>
            <AlertDialogDescription>
              データの競合が検出されました。どちらの変更を適用しますか？
            </AlertDialogDescription>
          </AlertDialogHeader>
          {selectedConflict && (
            <div className="space-y-4 my-4">
              <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded">
                <h5 className="font-medium mb-1">ローカルの変更</h5>
                <p className="text-sm text-muted-foreground">
                  作成日時:{' '}
                  {new Date(selectedConflict.localAction.createdAt).toLocaleString('ja-JP')}
                </p>
                <p className="text-sm mt-1">タイプ: {selectedConflict.localAction.actionType}</p>
              </div>
              {selectedConflict.remoteAction && (
                <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded">
                  <h5 className="font-medium mb-1">リモートの変更</h5>
                  <p className="text-sm text-muted-foreground">
                    作成日時:{' '}
                    {new Date(selectedConflict.remoteAction.createdAt).toLocaleString('ja-JP')}
                  </p>
                  <p className="text-sm mt-1">タイプ: {selectedConflict.remoteAction.actionType}</p>
                </div>
              )}
            </div>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel>キャンセル</AlertDialogCancel>
            <AlertDialogAction onClick={() => handleConflictResolution('local')}>
              ローカルを適用
            </AlertDialogAction>
            {selectedConflict?.remoteAction && (
              <AlertDialogAction
                onClick={() => handleConflictResolution('remote')}
                className="bg-green-600 hover:bg-green-700"
              >
                リモートを適用
              </AlertDialogAction>
            )}
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
