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
      // 繧ｪ繝ｳ繝ｩ繧､繝ｳ蠕ｩ蟶ｰ譎・      setShowBanner(true);
      const timer = setTimeout(() => {
        setShowBanner(false);
        setWasOffline(false);
      }, 5000);
      return () => clearTimeout(timer);
    }
  }, [isOnline, wasOffline]);

  const getLastSyncText = () => {
    if (!lastSyncedAt) return '譛ｪ蜷梧悄';
    return formatDistanceToNow(lastSyncedAt, {
      addSuffix: true,
      locale: ja,
    });
  };

  const pendingCount = pendingActions.length;

  return (
    <>
      {/* 繝倥ャ繝繝ｼ繝舌リ繝ｼ */}
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
                <span className="text-sm font-medium">繧ｪ繝ｳ繝ｩ繧､繝ｳ縺ｫ蠕ｩ蟶ｰ縺励∪縺励◆</span>
                {isSyncing && <span className="text-xs opacity-90">・亥酔譛滉ｸｭ...・・/span>}
              </>
            ) : (
              <>
                <WifiOff className="h-4 w-4" />
                <span className="text-sm font-medium">繧ｪ繝輔Λ繧､繝ｳ繝｢繝ｼ繝・/span>
                <span className="text-xs opacity-90">
                  ・亥､画峩縺ｯ菫晏ｭ倥＆繧後√が繝ｳ繝ｩ繧､繝ｳ譎ゅ↓蜷梧悄縺輔ｌ縺ｾ縺呻ｼ・                </span>
              </>
            )}
          </div>
        </div>
      )}

      {/* 蟶ｸ險ｭ繧､繝ｳ繧ｸ繧ｱ繝ｼ繧ｿ繝ｼ・郁ｩｳ邏ｰ縺ｯ SyncStatusIndicator 蛛ｴ縺ｧ陦ｨ遉ｺ・・*/}
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
                      譛邨ょ酔譛・ {getLastSyncText()}
                    </>
                  ) : (
                    <>
                      <WifiOff className="h-4 w-4 inline mr-2" />
                      繧ｪ繝輔Λ繧､繝ｳ縺ｧ縺呻ｼ医・繝・ム繝ｼ蜿ｳ荳翫・ SyncStatusIndicator 縺ｧ隧ｳ邏ｰ繧堤｢ｺ隱阪〒縺阪∪縺呻ｼ・                    </>
                  )}
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <div className="space-y-1 text-xs text-muted-foreground">
                  {isSyncing && <p>蜷梧悄荳ｭ縺ｧ縺吮ｦ</p>}
                  {pendingCount > 0 && <p>譛ｪ蜷梧悄繧｢繧ｯ繧ｷ繝ｧ繝ｳ: {pendingCount}莉ｶ</p>}
                  <p>隧ｳ邏ｰ縺ｪ繧ｹ繝・・繧ｿ繧ｹ縺ｯ繝倥ャ繝繝ｼ蜿ｳ荳翫・ SyncStatusIndicator 縺九ｉ遒ｺ隱阪〒縺阪∪縺吶・/p>
                </div>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>
      )}
    </>
  );
}
