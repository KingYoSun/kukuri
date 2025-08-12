import { useEffect, useCallback } from 'react';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import type { SaveOfflineActionRequest } from '@/types/offline';
import { OfflineActionType } from '@/types/offline';

/**
 * オフライン状態を監視し、オフライン機能を提供するフック
 */
export function useOffline() {
  const {
    isOnline,
    pendingActions,
    isSyncing,
    lastSyncedAt,
    saveOfflineAction,
    syncPendingActions,
    loadPendingActions,
  } = useOfflineStore();

  const { currentAccount } = useAuthStore();

  // コンポーネントマウント時に保留中のアクションを読み込む
  useEffect(() => {
    if (currentAccount?.npub) {
      loadPendingActions(currentAccount.npub);
    }
  }, [currentAccount?.npub, loadPendingActions]);

  // オンライン/オフライン状態の変化を監視
  useEffect(() => {
    const handleOnline = () => {
      toast.success('オンラインになりました。データを同期しています...');
      if (currentAccount?.npub && pendingActions.length > 0) {
        syncPendingActions(currentAccount.npub);
      }
    };

    const handleOffline = () => {
      toast.info('オフラインモードです。変更は後で同期されます。');
    };

    // 初期状態のチェック
    if (!isOnline) {
      handleOffline();
    }

    // イベントリスナーの設定
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [isOnline, currentAccount?.npub, pendingActions.length, syncPendingActions]);

  // 定期的な同期（5分ごと）
  useEffect(() => {
    if (!isOnline || !currentAccount?.npub) return;

    const interval = setInterval(() => {
      if (pendingActions.length > 0 && !isSyncing) {
        syncPendingActions(currentAccount.npub);
      }
    }, 5 * 60 * 1000); // 5分

    return () => clearInterval(interval);
  }, [isOnline, currentAccount?.npub, pendingActions.length, isSyncing, syncPendingActions]);

  // オフラインアクションを保存するヘルパー
  const saveAction = useCallback(
    async (actionType: OfflineActionType, targetId?: string, actionData?: Record<string, any>) => {
      if (!currentAccount?.npub) {
        throw new Error('User not authenticated');
      }

      const request: SaveOfflineActionRequest = {
        userPubkey: currentAccount.npub,
        actionType,
        targetId,
        actionData: actionData || {},
      };

      await saveOfflineAction(request);

      // オフライン時は通知を表示
      if (!isOnline) {
        toast.info('アクションが保存されました。オンライン時に同期されます。');
      }
    },
    [currentAccount?.npub, saveOfflineAction, isOnline]
  );

  // 手動同期トリガー
  const triggerSync = useCallback(async () => {
    if (!currentAccount?.npub) {
      toast.error('ログインが必要です');
      return;
    }

    if (!isOnline) {
      toast.warning('オフラインのため同期できません');
      return;
    }

    if (isSyncing) {
      toast.info('すでに同期中です');
      return;
    }

    if (pendingActions.length === 0) {
      toast.info('同期するアクションはありません');
      return;
    }

    try {
      await syncPendingActions(currentAccount.npub);
      toast.success('同期が完了しました');
    } catch (error) {
      toast.error('同期に失敗しました');
      errorHandler.log('Sync failed', error, {
        context: 'useOffline.triggerSync'
      });
    }
  }, [currentAccount?.npub, isOnline, isSyncing, pendingActions.length, syncPendingActions]);

  return {
    isOnline,
    isSyncing,
    pendingCount: pendingActions.length,
    lastSyncedAt,
    saveAction,
    triggerSync,
  };
}

/**
 * オフライン対応の楽観的更新を行うフック
 */
export function useOptimisticUpdate<T = any>() {
  const {
    applyOptimisticUpdate,
    confirmUpdate,
    rollbackUpdate,
  } = useOfflineStore();

  const apply = useCallback(
    async (
      entityType: string,
      entityId: string,
      originalData: T,
      updatedData: T,
      onSuccess?: () => void,
      onError?: (error: Error) => void
    ) => {
      try {
        // 楽観的更新を適用
        const updateId = await applyOptimisticUpdate(
          entityType as any,
          entityId,
          originalData,
          updatedData
        );

        // 実際のAPI呼び出しなどを行う
        if (onSuccess) {
          try {
            await onSuccess();
            // 成功したら確認
            await confirmUpdate(updateId);
          } catch (error) {
            // 失敗したらロールバック
            await rollbackUpdate(updateId);
            if (onError) {
              onError(error as Error);
            }
            throw error;
          }
        }

        return updateId;
      } catch (error) {
        errorHandler.log('Optimistic update failed', error, {
          context: 'useOptimisticUpdate.apply'
        });
        throw error;
      }
    },
    [applyOptimisticUpdate, confirmUpdate, rollbackUpdate]
  );

  return { apply };
}