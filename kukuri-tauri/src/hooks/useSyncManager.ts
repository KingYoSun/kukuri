import { useState, useCallback, useEffect } from 'react';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { syncEngine, type SyncResult, type SyncConflict } from '@/lib/sync/syncEngine';
import { toast } from 'sonner';

export interface SyncStatus {
  isSyncing: boolean;
  progress: number;
  totalItems: number;
  syncedItems: number;
  conflicts: SyncConflict[];
  lastSyncTime?: Date;
  error?: string;
}

export function useSyncManager() {
  const { 
    pendingActions, 
    isOnline, 
    lastSyncedAt,
    syncPendingActions,
    clearPendingActions,
    setSyncError,
    clearSyncError,
  } = useOfflineStore();
  
  const { currentAccount } = useAuthStore();
  
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({
    isSyncing: false,
    progress: 0,
    totalItems: 0,
    syncedItems: 0,
    conflicts: [],
    lastSyncTime: lastSyncedAt ? new Date(lastSyncedAt) : undefined,
  });

  /**
   * 手動同期トリガー
   */
  const triggerManualSync = useCallback(async () => {
    if (!isOnline) {
      toast.error('オフラインのため同期できません');
      return;
    }

    if (syncStatus.isSyncing) {
      toast.warning('同期処理が既に実行中です');
      return;
    }

    if (pendingActions.length === 0) {
      toast.info('同期するアクションがありません');
      return;
    }

    setSyncStatus(prev => ({
      ...prev,
      isSyncing: true,
      progress: 0,
      totalItems: pendingActions.length,
      syncedItems: 0,
      conflicts: [],
      error: undefined,
    }));

    try {
      // 差分同期を実行
      const result = await syncEngine.performDifferentialSync(pendingActions);
      
      // 同期結果を処理
      await processSyncResult(result);
      
      // 成功したアクションをクリア
      if (result.syncedActions.length > 0) {
        for (const action of result.syncedActions) {
          clearSyncError(action.localId);
        }
      }
      
      setSyncStatus(prev => ({
        ...prev,
        isSyncing: false,
        progress: 100,
        syncedItems: result.syncedActions.length,
        conflicts: result.conflicts,
        lastSyncTime: new Date(),
      }));
      
      // 競合がある場合は通知
      if (result.conflicts.length > 0) {
        toast.warning(`${result.conflicts.length}件の競合が検出されました`);
      } else {
        toast.success(`${result.syncedActions.length}件のアクションを同期しました`);
      }
      
    } catch (error) {
      console.error('同期エラー:', error);
      setSyncStatus(prev => ({
        ...prev,
        isSyncing: false,
        error: error instanceof Error ? error.message : '同期に失敗しました',
      }));
      toast.error('同期に失敗しました');
    }
  }, [isOnline, syncStatus.isSyncing, pendingActions, clearSyncError]);

  /**
   * 同期結果を処理
   */
  const processSyncResult = async (result: SyncResult) => {
    // 失敗したアクションにエラーをマーク
    for (const failedAction of result.failedActions) {
      setSyncError(failedAction.localId, '同期に失敗しました');
    }
    
    // 競合の手動解決が必要な場合
    const manualConflicts = result.conflicts.filter(c => c.resolution === 'manual');
    if (manualConflicts.length > 0) {
      // TODO: 競合解決UIを表示
      console.log('手動解決が必要な競合:', manualConflicts);
    }
    
    // Zustandストアの同期処理を呼び出し
    if (currentAccount?.npub) {
      await syncPendingActions(currentAccount.npub);
    }
  };

  /**
   * 競合を手動で解決
   */
  const resolveConflict = useCallback(async (
    conflict: SyncConflict, 
    resolution: 'local' | 'remote' | 'merge'
  ) => {
    conflict.resolution = resolution;
    
    try {
      if (resolution === 'local') {
        // ローカルのアクションを適用
        await syncEngine['applyAction'](conflict.localAction);
        toast.success('ローカルの変更を適用しました');
      } else if (resolution === 'remote' && conflict.remoteAction) {
        // リモートのアクションを適用
        await syncEngine['applyAction'](conflict.remoteAction);
        toast.success('リモートの変更を適用しました');
      } else if (resolution === 'merge' && conflict.mergedData) {
        // マージしたデータを適用
        // TODO: マージ適用ロジックを実装
        toast.success('変更をマージしました');
      }
      
      // 競合リストから削除
      setSyncStatus(prev => ({
        ...prev,
        conflicts: prev.conflicts.filter(c => c !== conflict),
      }));
    } catch (error) {
      console.error('競合解決エラー:', error);
      toast.error('競合の解決に失敗しました');
    }
  }, []);

  /**
   * 同期進捗の更新
   */
  const updateProgress = useCallback((syncedItems: number, totalItems: number) => {
    const progress = totalItems > 0 ? (syncedItems / totalItems) * 100 : 0;
    
    setSyncStatus(prev => ({
      ...prev,
      progress,
      syncedItems,
      totalItems,
    }));
  }, []);

  /**
   * 自動同期の設定
   */
  useEffect(() => {
    if (!isOnline || pendingActions.length === 0) {
      return;
    }

    // オンライン復帰時に自動同期
    const syncTimer = setTimeout(() => {
      triggerManualSync();
    }, 2000); // 2秒後に同期

    return () => clearTimeout(syncTimer);
  }, [isOnline]); // triggerManualSyncは依存配列に含めない（無限ループ防止）

  /**
   * 定期同期の設定
   */
  useEffect(() => {
    if (!isOnline) {
      return;
    }

    // 5分ごとに自動同期
    const interval = setInterval(() => {
      if (pendingActions.length > 0 && !syncStatus.isSyncing) {
        triggerManualSync();
      }
    }, 5 * 60 * 1000);

    return () => clearInterval(interval);
  }, [isOnline, pendingActions.length]); // triggerManualSyncとsyncStatus.isSyncingは依存配列に含めない

  return {
    syncStatus,
    triggerManualSync,
    resolveConflict,
    updateProgress,
    pendingActionsCount: pendingActions.length,
    isOnline,
  };
}