import React from 'react';
import { WifiOff, Wifi, Clock } from 'lucide-react';
import { useOfflineStore } from '@/stores/offlineStore';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { cn } from '@/lib/utils';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Badge } from '@/components/ui/badge';

export function OfflineIndicator() {
  const { isOnline, lastSyncedAt, pendingActions, isSyncing } = useOfflineStore();
  const [showBanner, setShowBanner] = React.useState(!isOnline);

  React.useEffect(() => {
    const wasOffline = !isOnline;
    setShowBanner(!isOnline);

    if (isOnline && wasOffline) {
      const timer = setTimeout(() => {
        setShowBanner(false);
      }, 5000);
      return () => clearTimeout(timer);
    }
  }, [isOnline]);

  const getLastSyncText = () => {
    if (!lastSyncedAt) return '未同期';
    return formatDistanceToNow(lastSyncedAt, { 
      addSuffix: true, 
      locale: ja 
    });
  };

  const pendingCount = pendingActions.length;

  if (isOnline && !showBanner && pendingCount === 0) {
    return null;
  }

  return (
    <>
      {/* ヘッダーバナー（オフライン時のみ表示） */}
      {showBanner && (
        <div
          className={cn(
            'fixed top-0 left-0 right-0 z-50 px-4 py-2 text-center transition-all duration-300',
            isOnline
              ? 'bg-green-500 text-white'
              : 'bg-orange-500 text-white'
          )}
        >
          <div className="flex items-center justify-center gap-2">
            {isOnline ? (
              <>
                <Wifi className="h-4 w-4" />
                <span className="text-sm font-medium">
                  オンラインに復帰しました
                </span>
                {isSyncing && (
                  <span className="text-xs opacity-90">
                    （同期中...）
                  </span>
                )}
              </>
            ) : (
              <>
                <WifiOff className="h-4 w-4" />
                <span className="text-sm font-medium">
                  オフラインモード
                </span>
                <span className="text-xs opacity-90">
                  （変更は保存され、オンライン時に同期されます）
                </span>
              </>
            )}
          </div>
        </div>
      )}

      {/* 永続的なステータスインジケーター */}
      <div className="fixed bottom-4 right-4 z-40">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <div
                className={cn(
                  'flex items-center gap-2 rounded-full px-3 py-1.5 shadow-lg transition-all',
                  isOnline
                    ? pendingCount > 0
                      ? 'bg-yellow-100 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-700'
                      : 'bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-700'
                    : 'bg-orange-100 dark:bg-orange-900/20 border border-orange-300 dark:border-orange-700'
                )}
              >
                {isOnline ? (
                  pendingCount > 0 ? (
                    <>
                      <Clock className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
                      <Badge variant="secondary" className="h-5 px-1.5">
                        {pendingCount}
                      </Badge>
                    </>
                  ) : (
                    <Wifi className="h-4 w-4 text-green-600 dark:text-green-400" />
                  )
                ) : (
                  <>
                    <WifiOff className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                    {pendingCount > 0 && (
                      <Badge variant="secondary" className="h-5 px-1.5">
                        {pendingCount}
                      </Badge>
                    )}
                  </>
                )}
              </div>
            </TooltipTrigger>
            <TooltipContent side="left" className="max-w-xs">
              <div className="space-y-1">
                <div className="font-medium">
                  {isOnline ? 'オンライン' : 'オフライン'}
                </div>
                {pendingCount > 0 && (
                  <div className="text-sm">
                    {pendingCount}件の未同期アクション
                  </div>
                )}
                <div className="text-xs text-muted-foreground">
                  最終同期: {getLastSyncText()}
                </div>
              </div>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </>
  );
}