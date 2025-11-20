import React from 'react';
import { WifiOff, Wifi } from 'lucide-react';
import { useOfflineStore } from '@/stores/offlineStore';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { cn } from '@/lib/utils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

export function OfflineIndicator() {
  const { isOnline, lastSyncedAt, pendingActions, isSyncing } = useOfflineStore();
  const [showBanner, setShowBanner] = React.useState(!isOnline);
  const [wasOffline, setWasOffline] = React.useState(!isOnline);

  React.useEffect(() => {
    if (!isOnline) {
      setShowBanner(true);
      setWasOffline(true);
    } else if (wasOffline) {
      // オンライン復帰時に一時的にバナーを表示
      setShowBanner(true);
      const timer = setTimeout(() => {
        setShowBanner(false);
        setWasOffline(false);
      }, 5000);
      return () => clearTimeout(timer);
    }
  }, [isOnline, wasOffline]);

  const getLastSyncText = () => {
    if (!lastSyncedAt) return '未同期';
    return formatDistanceToNow(lastSyncedAt, {
      addSuffix: true,
      locale: ja,
    });
  };

  const pendingCount = pendingActions.length;

  return (
    <>
      {/* ヘッダーバナー */}
      {showBanner && (
        <div
          className={cn(
            'fixed top-0 left-0 right-0 z-50 px-4 py-2 text-center transition-all duration-300',
            isOnline ? 'bg-green-500 text-white' : 'bg-orange-500 text-white',
          )}
          data-testid="offline-indicator-banner"
        >
          <div className="flex items-center justify-center gap-2">
            {isOnline ? (
              <>
                <Wifi className="h-4 w-4" />
                <span className="text-sm font-medium">オンラインに復帰しました</span>
                {isSyncing && <span className="text-xs opacity-90">同期中...</span>}
              </>
            ) : (
              <>
                <WifiOff className="h-4 w-4" />
                <span className="text-sm font-medium">オフラインモード</span>
                <span className="text-xs opacity-90">
                  変更は保存され、オンライン時に同期されます
                </span>
              </>
            )}
          </div>
        </div>
      )}

      {/* 常設インジケーター。詳細は SyncStatusIndicator 側で表示 */}
      {(pendingCount > 0 || !isOnline || isSyncing) && (
        <div
          className="fixed bottom-4 right-4 left-4 mx-auto max-w-sm z-40"
          data-testid="offline-indicator-container"
        >
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  className={cn(
                    'w-full rounded-full px-4 py-2 shadow-lg border transition-colors text-sm',
                    isOnline
                      ? 'bg-white text-gray-700 border-gray-200'
                      : 'bg-orange-100 text-orange-800 border-orange-200',
                  )}
                  data-testid="offline-indicator-pill"
                >
                  {isOnline ? (
                    <>
                      <Wifi className="h-4 w-4 inline mr-2" />
                      最終同期 {getLastSyncText()}
                    </>
                  ) : (
                    <>
                      <WifiOff className="h-4 w-4 inline mr-2" />
                      オフラインです（ヘッダー右上の SyncStatusIndicator で詳細を確認できます）
                    </>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <div className="space-y-1 text-xs text-muted-foreground">
                  {isSyncing && <p>同期中です…</p>}
                  {pendingCount > 0 && <p>未同期アクション: {pendingCount}件</p>}
                  <p>詳細なステータスはヘッダー右上の SyncStatusIndicator から確認できます</p>
                </div>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      )}
    </>
  );
}
