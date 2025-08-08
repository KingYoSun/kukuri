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
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { 
  RefreshCw, 
  AlertCircle, 
  CheckCircle, 
  Clock,
  Wifi,
  WifiOff,
  AlertTriangle,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import type { SyncConflict } from '@/lib/sync/syncEngine';

export function SyncStatusIndicator() {
  const {
    syncStatus,
    triggerManualSync,
    resolveConflict,
    pendingActionsCount,
    isOnline,
  } = useSyncManager();

  const [selectedConflict, setSelectedConflict] = React.useState<SyncConflict | null>(null);
  const [showConflictDialog, setShowConflictDialog] = React.useState(false);

  const handleConflictResolution = (resolution: 'local' | 'remote' | 'merge') => {
    if (selectedConflict) {
      resolveConflict(selectedConflict, resolution);
      setShowConflictDialog(false);
      setSelectedConflict(null);
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

  return (
    <>
      <Popover>
        <PopoverTrigger asChild>
          <Button
            variant="ghost"
            size="sm"
            className="gap-2"
          >
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
                      <p className="font-medium">
                        {conflict.localAction.actionType}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        クリックして解決
                      </p>
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
                <p className="text-sm text-red-600 dark:text-red-400">
                  {syncStatus.error}
                </p>
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
                  作成日時: {new Date(selectedConflict.localAction.createdAt).toLocaleString('ja-JP')}
                </p>
                <p className="text-sm mt-1">
                  タイプ: {selectedConflict.localAction.actionType}
                </p>
              </div>
              {selectedConflict.remoteAction && (
                <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded">
                  <h5 className="font-medium mb-1">リモートの変更</h5>
                  <p className="text-sm text-muted-foreground">
                    作成日時: {new Date(selectedConflict.remoteAction.createdAt).toLocaleString('ja-JP')}
                  </p>
                  <p className="text-sm mt-1">
                    タイプ: {selectedConflict.remoteAction.actionType}
                  </p>
                </div>
              )}
            </div>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel>キャンセル</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => handleConflictResolution('local')}
            >
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