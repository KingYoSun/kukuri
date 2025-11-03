import { useP2P } from '@/hooks/useP2P';
import { useEffect, useCallback, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
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
  Loader2,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

export function P2PStatus() {
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
            接続中
          </Badge>
        );
      case 'connecting':
        return <Badge variant="secondary">接続中...</Badge>;
      case 'error':
        return <Badge variant="destructive">エラー</Badge>;
      default:
        return <Badge variant="outline">未接続</Badge>;
    }
  };

  // 接続中のピア数を計算
  const connectedPeerCount = peers.filter((p) => p.connection_status === 'connected').length;

  const lastUpdatedLabel = useMemo(() => {
    if (!lastStatusFetchedAt) {
      return '未取得';
    }
    return formatDistanceToNow(lastStatusFetchedAt, { addSuffix: true, locale: ja });
  }, [lastStatusFetchedAt]);

  const nextRefreshLabel = useMemo(() => {
    if (statusBackoffMs >= 600_000) {
      return '約10分';
    }
    if (statusBackoffMs >= 300_000) {
      return '約5分';
    }
    if (statusBackoffMs >= 120_000) {
      return '約2分';
    }
    return '約30秒';
  }, [statusBackoffMs]);

  return (
    <Card className="w-full">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium">P2P ネットワーク</CardTitle>
          {getConnectionIcon()}
        </div>
        <CardDescription className="text-xs">分散型ネットワーク接続状態</CardDescription>
        <div className="mt-2 flex items-center justify-between text-[11px] text-muted-foreground">
          <span>最終更新: {lastUpdatedLabel}</span>
          <span>次回再取得: {nextRefreshLabel}</span>
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
                更新中…
              </>
            ) : (
              '再取得'
            )}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 接続状態 */}
        <div className="flex items-center justify-between">
          <span className="text-sm text-muted-foreground">状態</span>
          {getConnectionBadge()}
        </div>

        {/* エラー表示 */}
        {(error || statusError) && (
          <div className="bg-red-50 dark:bg-red-950 rounded-md p-3">
            <div className="flex items-start space-x-2">
              <AlertCircleIcon className="h-4 w-4 text-red-500 mt-0.5" />
              <div className="flex-1">
                {error && (
                  <p className="text-xs text-red-600 dark:text-red-400">{error}</p>
                )}
                {statusError && (
                  <p className="text-xs text-red-600 dark:text-red-400">
                    状態取得エラー: {statusError}
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
                      閉じる
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
                      再取得
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
                <span className="text-muted-foreground">ノードID</span>
              </div>
              <p className="text-xs font-mono break-all bg-muted/50 rounded px-2 py-1">
                {nodeId?.slice(0, 16)}...
              </p>
            </div>

            {/* ピア情報 */}
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2 text-sm">
                <UsersIcon className="h-3 w-3 text-muted-foreground" />
                <span className="text-muted-foreground">接続ピア</span>
              </div>
              <span className="text-sm font-medium">{connectedPeerCount}</span>
            </div>

            {/* メトリクスサマリ */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Gossipメトリクス</span>
                <Button
                  variant="secondary"
                  size="sm"
                  className="h-6 text-xs"
                  onClick={handleRefresh}
                  disabled={isRefreshingStatus}
                >
                  更新
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
                  <span className="text-muted-foreground">参加中のトピック</span>
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
                  <p className="text-xs text-muted-foreground">ネットワークアドレス</p>
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
            <p className="text-sm text-muted-foreground">P2Pネットワークに接続していません</p>
          </div>
        )}

        {connectionStatus === 'connecting' && (
          <div className="text-center py-4">
            <CircleIcon className="h-8 w-8 text-yellow-500 animate-pulse mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">ネットワークに接続中...</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
