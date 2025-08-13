import React from 'react';
import { WifiOff, Wifi } from 'lucide-react';
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


export function OfflineIndicator() {
  const { isOnline, lastSyncedAt, pendingActions, isSyncing } = useOfflineStore();
  const [showBanner, setShowBanner] = React.useState(!isOnline);
  const [wasOffline, setWasOffline] = React.useState(!isOnline);

  React.useEffect(() => {
    if (!isOnline) {
      setShowBanner(true);
      setWasOffline(true);
    } else if (wasOffline) {
      // オンライン復帰時
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
      locale: ja 
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

      {/* 常設インジケーター */}
      {(pendingCount > 0 || !isOnline || isSyncing) && (
        <div className="fixed bottom-4 right-4 z-40">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  className={cn(
                    "relative flex items-center justify-center h-12 w-12 rounded-full shadow-lg transition-all",
                    isOnline 
                      ? "bg-white border-2 border-gray-200"
                      : "bg-orange-100 border-2 border-orange-300"
                  )}
                >
                  {isOnline ? (
                    <Wifi className="h-5 w-5 text-gray-600" />
                  ) : (
                    <WifiOff className="h-5 w-5 text-orange-600" />
                  )}
                  {pendingCount > 0 && (
                    <span className="absolute -top-1 -right-1 flex h-5 w-5 items-center justify-center rounded-full bg-red-500 text-xs text-white">
                      {pendingCount}
                    </span>
                  )}
                  {isSyncing && (
                    <span className="absolute -bottom-1 -right-1 flex h-3 w-3">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-3 w-3 bg-blue-500"></span>
                    </span>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <div className="space-y-1">
                  <p className="text-sm font-medium">
                    {isOnline ? 'オンライン' : 'オフライン'}
                  </p>
                  {isSyncing && (
                    <p className="text-xs text-gray-500">（同期中...）</p>
                  )}
                  <p className="text-xs text-gray-500">
                    最終同期: {getLastSyncText()}
                  </p>
                  {pendingCount > 0 && (
                    <p className="text-xs text-gray-500">
                      未同期: {pendingCount}件
                    </p>
                  )}
                </div>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      )}
    </>
  );
}