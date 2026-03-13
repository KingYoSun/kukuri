import React from 'react';
import { useTranslation } from 'react-i18next';
import { WifiOff, Wifi } from 'lucide-react';
import { useOfflineStore } from '@/stores/offlineStore';
import { formatDistanceToNow } from 'date-fns';
import { cn } from '@/lib/utils';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { getDateFnsLocale } from '@/i18n';

export function OfflineIndicator() {
  const { t } = useTranslation();
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
    if (!lastSyncedAt) return t('offline.notSynced');
    return formatDistanceToNow(lastSyncedAt, {
      addSuffix: true,
      locale: getDateFnsLocale(),
    });
  };

  const pendingCount = pendingActions.length;
  const shouldShowPill = pendingCount > 0 || !isOnline || isSyncing;

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
                <span className="text-sm font-medium">{t('offline.onlineRestored')}</span>
                {isSyncing && <span className="text-xs opacity-90">{t('offline.syncing')}</span>}
              </>
            ) : (
              <>
                <WifiOff className="h-4 w-4" />
                <span className="text-sm font-medium">{t('offline.offlineMode')}</span>
                <span className="text-xs opacity-90">{t('offline.changesSaved')}</span>
              </>
            )}
          </div>
        </div>
      )}

      {/* 常設インジケーター。詳細は SyncStatusIndicator 側で表示 */}
      {shouldShowPill && (
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
                      ? 'bg-card text-card-foreground border-border'
                      : 'bg-orange-100 text-orange-800 border-orange-200 dark:bg-orange-900/30 dark:text-orange-100 dark:border-orange-700/50',
                  )}
                  data-testid="offline-indicator-pill"
                >
                  {isOnline ? (
                    <>
                      <Wifi className="h-4 w-4 inline mr-2" />
                      {t('offline.lastSync', { time: getLastSyncText() })}
                    </>
                  ) : (
                    <>
                      <WifiOff className="h-4 w-4 inline mr-2" />
                      {t('offline.offlineStatus')}
                    </>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <div className="space-y-1 text-xs text-muted-foreground">
                  {isSyncing && <p>{t('offline.syncingStatus')}</p>}
                  {pendingCount > 0 && (
                    <p>{t('offline.pendingActions', { count: pendingCount })}</p>
                  )}
                  <p>{t('offline.checkDetails')}</p>
                </div>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      )}
    </>
  );
}
