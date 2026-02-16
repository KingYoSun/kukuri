import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Wifi, WifiOff } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

/**
 * リアルタイム更新インジケーター
 * ネットワーク接続状態とリアルタイム更新の状態を表示
 */
export function RealtimeIndicator() {
  const { t } = useTranslation();
  const [isOnline, setIsOnline] = useState(navigator.onLine);
  const [lastUpdate, setLastUpdate] = useState(new Date());

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  // リアルタイム更新イベントをリスニング
  useEffect(() => {
    const updateIndicator = () => {
      setLastUpdate(new Date());
    };

    // カスタムイベントをリスニング
    window.addEventListener('realtime-update', updateIndicator);

    return () => {
      window.removeEventListener('realtime-update', updateIndicator);
    };
  }, []);

  const getRelativeTime = () => {
    const now = new Date();
    const diff = now.getTime() - lastUpdate.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);

    if (seconds < 10) return t('realtime.connecting');
    if (seconds < 60) return t('realtime.secondsAgo', { seconds });
    if (minutes < 60) return t('realtime.minutesAgo', { minutes });
    return t('realtime.overAnHourAgo');
  };

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className={cn(
              'flex items-center gap-2 px-3 py-1 rounded-full text-xs font-medium transition-colors',
              isOnline
                ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400'
                : 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
            )}
          >
            {isOnline ? <Wifi className="h-3 w-3" /> : <WifiOff className="h-3 w-3" />}
            <span>{isOnline ? getRelativeTime() : t('realtime.offline')}</span>
          </div>
        </TooltipTrigger>
        <TooltipContent>
          <p>
            {isOnline
              ? t('realtime.realtimeUpdate', { time: lastUpdate.toLocaleTimeString() })
              : t('realtime.noInternetConnection')}
          </p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
