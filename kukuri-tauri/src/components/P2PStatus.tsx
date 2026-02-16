import { useTranslation } from 'react-i18next';
import { useP2P } from '@/hooks/useP2P';
import { useEffect, useCallback, useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  WifiIcon,
  WifiOffIcon,
  UsersIcon,
  ServerIcon,
  NetworkIcon,
  AlertCircleIcon,
  CircleIcon,
  ChevronDown,
  Loader2,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { getDateFnsLocale } from '@/i18n';

export function P2PStatus() {
  const { t } = useTranslation();
  const {
    initialized,
    nodeId,
    nodeAddr,
    activeTopics,
    peers,
    connectionStatus,
    error,
    clearError,
    refreshStatus,
    metricsSummary,
    statusError,
    statusBackoffMs,
    lastStatusFetchedAt,
    isRefreshingStatus,
  } = useP2P();
  const [detailsOpen, setDetailsOpen] = useState(true);

  useEffect(() => {
    if (!initialized || isRefreshingStatus) {
      return;
    }
    if (lastStatusFetchedAt === null) {
      void refreshStatus();
    }
  }, [initialized, lastStatusFetchedAt, isRefreshingStatus, refreshStatus]);

  useEffect(() => {
    if (!initialized || lastStatusFetchedAt === null) {
      return;
    }

    const timer = setTimeout(() => {
      void refreshStatus();
    }, statusBackoffMs);

    return () => clearTimeout(timer);
  }, [initialized, statusBackoffMs, lastStatusFetchedAt, refreshStatus]);

  const handleRefresh = useCallback(async () => {
    if (isRefreshingStatus) {
      return;
    }
    try {
      await refreshStatus();
    } catch {
      // 既に refreshStatus 内でロギング済み
    }
  }, [refreshStatus, isRefreshingStatus]);

  const metrics = metricsSummary;

  // 接続状態のアイコンとカラーを取得
  const getConnectionIcon = () => {
    switch (connectionStatus) {
      case 'connected':
        return <WifiIcon className="h-4 w-4 text-green-500" />;
      case 'connecting':
        return <CircleIcon className="h-4 w-4 text-yellow-500 animate-pulse" />;
      case 'error':
        return <AlertCircleIcon className="h-4 w-4 text-red-500" />;
      default:
        return <WifiOffIcon className="h-4 w-4 text-gray-500" />;
    }
  };

  const getConnectionBadge = () => {
    switch (connectionStatus) {
      case 'connected':
        return (
          <Badge variant="default" className="bg-green-500">
            {t('p2pStatus.connected')}
          </Badge>
        );
      case 'connecting':
        return <Badge variant="secondary">{t('p2pStatus.connecting')}</Badge>;
      case 'error':
        return <Badge variant="destructive">{t('p2pStatus.error')}</Badge>;
      default:
        return <Badge variant="outline">{t('p2pStatus.disconnected')}</Badge>;
    }
  };

  // 接続中のピア数を計算
  const connectedPeerCount = peers.filter((p) => p.connection_status === 'connected').length;

  const lastUpdatedLabel = useMemo(() => {
    if (!lastStatusFetchedAt) {
      return t('p2pStatus.notFetched');
    }
    return formatDistanceToNow(lastStatusFetchedAt, {
      addSuffix: true,
      locale: getDateFnsLocale(),
    });
  }, [lastStatusFetchedAt, t]);

  const nextRefreshLabel = useMemo(() => {
    if (statusBackoffMs >= 600_000) {
      return t('p2pStatus.about10min');
    }
    if (statusBackoffMs >= 300_000) {
      return t('p2pStatus.about5min');
    }
    if (statusBackoffMs >= 120_000) {
      return t('p2pStatus.about2min');
    }
    return t('p2pStatus.about30sec');
  }, [statusBackoffMs, t]);

  return (
    <Collapsible open={detailsOpen} onOpenChange={setDetailsOpen}>
      <Card className="w-full">
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CollapsibleTrigger asChild>
              <Button variant="ghost" size="sm" className="h-auto px-1 text-sm font-semibold">
                <ChevronDown
                  className={`h-4 w-4 transition-transform ${
                    detailsOpen ? 'rotate-0' : '-rotate-90'
                  }`}
                />
                {t('p2pStatus.title')}
              </Button>
            </CollapsibleTrigger>
            {getConnectionIcon()}
          </div>
          <CardDescription className="text-xs">{t('p2pStatus.description')}</CardDescription>
          <div className="mt-2 flex items-center justify-between text-[11px] text-muted-foreground">
            <span>
              {t('p2pStatus.lastUpdated')}: {lastUpdatedLabel}
            </span>
            <span>
              {t('p2pStatus.nextRefresh')}: {nextRefreshLabel}
            </span>
          </div>
          <div className="mt-2 flex justify-end">
            <Button
              variant="outline"
              size="sm"
              className="h-7"
              onClick={handleRefresh}
              disabled={isRefreshingStatus}
            >
              {isRefreshingStatus ? (
                <>
                  <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                  {t('p2pStatus.refreshing')}
                </>
              ) : (
                t('p2pStatus.refresh')
              )}
            </Button>
          </div>
        </CardHeader>
        <CollapsibleContent>
          <CardContent className="space-y-4">
            {/* 接続状態 */}
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">{t('p2pStatus.status')}</span>
              {getConnectionBadge()}
            </div>

            {/* エラー表示 */}
            {(error || statusError) && (
              <div className="bg-red-50 dark:bg-red-950 rounded-md p-3">
                <div className="flex items-start space-x-2">
                  <AlertCircleIcon className="h-4 w-4 text-red-500 mt-0.5" />
                  <div className="flex-1">
                    {error && <p className="text-xs text-red-600 dark:text-red-400">{error}</p>}
                    {statusError && (
                      <p className="text-xs text-red-600 dark:text-red-400">
                        {t('p2pStatus.statusFetchError')}: {statusError}
                      </p>
                    )}
                    <div className="mt-1 flex flex-wrap gap-2">
                      {error && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 text-xs"
                          onClick={clearError}
                        >
                          {t('p2pStatus.close')}
                        </Button>
                      )}
                      {statusError && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 text-xs"
                          onClick={handleRefresh}
                          disabled={isRefreshingStatus}
                        >
                          {t('p2pStatus.refresh')}
                        </Button>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            )}

            {initialized && connectionStatus === 'connected' && (
              <>
                <Separator />

                {/* ノード情報 */}
                <div className="space-y-2">
                  <div className="flex items-center space-x-2 text-xs">
                    <ServerIcon className="h-3 w-3 text-muted-foreground" />
                    <span className="text-muted-foreground">{t('p2pStatus.nodeId')}</span>
                  </div>
                  <p className="text-xs font-mono break-all bg-muted/50 rounded px-2 py-1">
                    {nodeId?.slice(0, 16)}...
                  </p>
                </div>

                {/* ピア情報 */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-2 text-sm">
                    <UsersIcon className="h-3 w-3 text-muted-foreground" />
                    <span className="text-muted-foreground">{t('p2pStatus.connectedPeers')}</span>
                  </div>
                  <span className="text-sm font-medium">{connectedPeerCount}</span>
                </div>

                {/* メトリクスサマリ */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">
                      {t('p2pStatus.gossipMetrics')}
                    </span>
                    <Button
                      variant="secondary"
                      size="sm"
                      className="h-6 text-xs"
                      onClick={handleRefresh}
                      disabled={isRefreshingStatus}
                    >
                      {t('p2pStatus.update')}
                    </Button>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="flex items-center justify-between">
                      <span>Join</span>
                      <Badge variant="outline">{metrics?.joins ?? 0}</Badge>
                    </div>
                    <div className="flex items-center justify-between">
                      <span>Leave</span>
                      <Badge variant="outline">{metrics?.leaves ?? 0}</Badge>
                    </div>
                    <div className="flex items-center justify-between">
                      <span>Broadcast</span>
                      <Badge variant="outline">{metrics?.broadcasts_sent ?? 0}</Badge>
                    </div>
                    <div className="flex items-center justify-between">
                      <span>Received</span>
                      <Badge variant="outline">{metrics?.messages_received ?? 0}</Badge>
                    </div>
                  </div>
                </div>

                {/* アクティブトピック */}
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2 text-sm">
                      <NetworkIcon className="h-3 w-3 text-muted-foreground" />
                      <span className="text-muted-foreground">{t('p2pStatus.joinedTopics')}</span>
                    </div>
                    <span className="text-sm font-medium">{activeTopics.length}</span>
                  </div>

                  {activeTopics.length > 0 && (
                    <ScrollArea className="h-24 w-full rounded-md border">
                      <div className="p-2 space-y-1">
                        {activeTopics.map((topic) => (
                          <div
                            key={topic.topic_id}
                            className="flex items-center justify-between text-xs py-1"
                          >
                            <span className="truncate flex-1 font-mono">
                              {topic.topic_id.slice(0, 8)}...
                            </span>
                            <div className="flex items-center space-x-2 ml-2">
                              <Badge variant="secondary" className="h-5 text-xs">
                                <UsersIcon className="h-2.5 w-2.5 mr-1" />
                                {topic.peer_count}
                              </Badge>
                              <Badge variant="outline" className="h-5 text-xs">
                                {topic.message_count} msgs
                              </Badge>
                            </div>
                          </div>
                        ))}
                      </div>
                    </ScrollArea>
                  )}
                </div>

                {/* ネットワークアドレス */}
                {nodeAddr && (
                  <>
                    <Separator />
                    <div className="space-y-2">
                      <p className="text-xs text-muted-foreground">
                        {t('p2pStatus.networkAddress')}
                      </p>
                      <code className="text-xs font-mono break-all bg-muted/50 rounded px-2 py-1 block">
                        {nodeAddr}
                      </code>
                    </div>
                  </>
                )}
              </>
            )}

            {!initialized && connectionStatus === 'disconnected' && (
              <div className="text-center py-4">
                <WifiOffIcon className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">{t('p2pStatus.notConnected')}</p>
              </div>
            )}

            {connectionStatus === 'connecting' && (
              <div className="text-center py-4">
                <CircleIcon className="h-8 w-8 text-yellow-500 animate-pulse mx-auto mb-2" />
                <p className="text-sm text-muted-foreground">
                  {t('p2pStatus.connectingToNetwork')}
                </p>
              </div>
            )}
          </CardContent>
        </CollapsibleContent>
      </Card>
    </Collapsible>
  );
}
