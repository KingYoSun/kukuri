import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import { errorHandler } from '@/lib/errorHandler';

export class OfflineSyncService {
  private syncInterval: NodeJS.Timeout | null = null;
  private retryTimeout: NodeJS.Timeout | null = null;
  private networkListener: (() => void) | null = null;

  /**
   * オフライン同期サービスの初期化
   */
  initialize() {
    // ネットワーク状態の監視
    this.setupNetworkListener();
    
    // 定期同期の開始
    this.startPeriodicSync();
    
    // アプリ起動時の初期同期
    this.performInitialSync();
  }

  /**
   * ネットワーク状態の監視設定
   */
  private setupNetworkListener() {
    const handleOnline = () => {
      const offlineStore = useOfflineStore.getState();
      offlineStore.setOnlineStatus(true);
      
      // オンライン復帰時に即座に同期を試行
      this.triggerSync();
    };

    const handleOffline = () => {
      const offlineStore = useOfflineStore.getState();
      offlineStore.setOnlineStatus(false);
      
      // 同期インターバルを停止
      this.stopPeriodicSync();
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    this.networkListener = () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }

  /**
   * 定期同期の開始
   */
  private startPeriodicSync() {
    // 既存のインターバルをクリア
    this.stopPeriodicSync();
    
    // 30秒ごとに同期を試行
    this.syncInterval = setInterval(() => {
      const { isOnline } = useOfflineStore.getState();
      if (isOnline) {
        this.triggerSync();
      }
    }, 30000);
  }

  /**
   * 定期同期の停止
   */
  private stopPeriodicSync() {
    if (this.syncInterval) {
      clearInterval(this.syncInterval);
      this.syncInterval = null;
    }
  }

  /**
   * 初期同期の実行
   */
  private async performInitialSync() {
    const authStore = useAuthStore.getState();
    const offlineStore = useOfflineStore.getState();
    
    if (!authStore.currentAccount?.pubkey) {
      return;
    }
    
    // 保存済みの未同期アクションを読み込み
    await offlineStore.loadPendingActions(authStore.currentAccount.pubkey);
    
    // オンラインの場合は同期を実行
    if (offlineStore.isOnline) {
      await this.triggerSync();
    }
    
    // 期限切れキャッシュのクリーンアップ
    await offlineStore.cleanupExpiredCache();
  }

  /**
   * 同期の実行
   */
  async triggerSync() {
    const authStore = useAuthStore.getState();
    const offlineStore = useOfflineStore.getState();
    
    if (!authStore.currentAccount?.pubkey || !offlineStore.isOnline || offlineStore.isSyncing) {
      return;
    }
    
    const pendingCount = offlineStore.pendingActions.length;
    if (pendingCount === 0) {
      return;
    }
    
    console.log(`Starting sync for ${pendingCount} pending actions`);
    
    try {
      await offlineStore.syncPendingActions(authStore.currentAccount.pubkey);
      console.log('Sync completed successfully');
      
      // 成功時はリトライタイマーをクリア
      if (this.retryTimeout) {
        clearTimeout(this.retryTimeout);
        this.retryTimeout = null;
      }
    } catch (error) {
      errorHandler.log('Sync failed', error, {
        context: 'OfflineSyncService.sync'
      });
      
      // エラー時は指数バックオフでリトライ
      this.scheduleRetry();
    }
  }

  /**
   * リトライのスケジューリング
   */
  private scheduleRetry() {
    if (this.retryTimeout) {
      return;
    }
    
    // 最初は5秒後、その後は倍々で増やす（最大5分）
    const retryDelay = Math.min(5000 * Math.pow(2, this.getRetryCount()), 300000);
    
    this.retryTimeout = setTimeout(() => {
      this.retryTimeout = null;
      this.triggerSync();
    }, retryDelay);
  }

  /**
   * リトライ回数の取得
   */
  private getRetryCount(): number {
    const offlineStore = useOfflineStore.getState();
    return offlineStore.syncErrors.size;
  }

  /**
   * クリーンアップ
   */
  cleanup() {
    this.stopPeriodicSync();
    
    if (this.retryTimeout) {
      clearTimeout(this.retryTimeout);
      this.retryTimeout = null;
    }
    
    if (this.networkListener) {
      this.networkListener();
      this.networkListener = null;
    }
  }
}

// シングルトンインスタンス
export const offlineSyncService = new OfflineSyncService();