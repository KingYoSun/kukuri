import { useTranslation } from 'react-i18next';
import { useState, useCallback, useEffect } from 'react';
import { p2pApi } from '@/lib/api/p2p';
import type { GossipMetricsSection, P2PMetrics } from '@/lib/api/p2p';
import { useP2P } from '@/hooks/useP2P';
import { useNostrSubscriptions } from '@/hooks/useNostrSubscriptions';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { SendIcon, NetworkIcon, TrashIcon, WifiIcon, WifiOffIcon } from 'lucide-react';
import { errorHandler } from '@/lib/errorHandler';

const formatPercent = (value: number) => {
  if (!Number.isFinite(value)) {
    return '0%';
  }
  return `${(value * 100).toFixed(1)}%`;
};

export function P2PDebugPanel() {
  const { t } = useTranslation();
  const {
    nodeId,
    nodeAddr,
    activeTopics,
    peers,
    connectionStatus,
    error,
    joinTopic,
    leaveTopic,
    broadcast,
    clearError,
  } = useP2P();

  const {
    subscriptions,
    isLoading: isSubscriptionLoading,
    error: subscriptionError,
    refresh: refreshSubscriptions,
  } = useNostrSubscriptions();

  const [newTopicId, setNewTopicId] = useState('');
  const [selectedTopic, setSelectedTopic] = useState('');
  const [messageContent, setMessageContent] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [metrics, setMetrics] = useState<P2PMetrics | null>(null);

  const formatTimestamp = (value: number | null | undefined) => {
    if (!value) {
      return '—';
    }
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? '—' : date.toLocaleTimeString();
  };

  const bootstrapSourceLabel = (source: string | null | undefined) => {
    switch (source) {
      case 'env':
        return t('p2pDebug.sourceEnv');
      case 'user':
        return t('p2pDebug.sourceUser');
      case 'bundle':
        return t('p2pDebug.sourceBundle');
      case 'fallback':
        return t('p2pDebug.sourceFallback');
      case 'none':
        return t('p2pDebug.sourceNone');
      default:
        return t('p2pDebug.sourceNotApplied');
    }
  };

  const renderMetricCard = (
    label: string,
    total: number,
    detail?: GossipMetricsSection['join_details'],
  ) => (
    <div className="rounded-md border p-2 space-y-1">
      <div className="flex items-center justify-between">
        <span>{label}</span>
        <Badge variant="outline">{total}</Badge>
      </div>
      <div className="grid gap-0.5 text-[10px] text-muted-foreground">
        <span>
          {t('p2pDebug.failures')}: {detail?.failures ?? 0}
        </span>
        <span>
          {t('p2pDebug.lastSuccess')}: {formatTimestamp(detail?.last_success_ms)}
        </span>
        <span>
          {t('p2pDebug.lastFailure')}: {formatTimestamp(detail?.last_failure_ms)}
        </span>
      </div>
    </div>
  );

  // ログを追加
  const addLog = useCallback((message: string) => {
    const timestamp = new Date().toISOString();
    setLogs((prev) => [`[${timestamp}] ${message}`, ...prev].slice(0, 100));
  }, []);

  // トピック参加
  const handleJoinTopic = useCallback(async () => {
    if (!newTopicId.trim()) return;

    setIsLoading(true);
    addLog(`Joining topic: ${newTopicId}`);

    try {
      await joinTopic(newTopicId.trim());
      addLog(`Successfully joined topic: ${newTopicId}`);
      setNewTopicId('');
      setSelectedTopic(newTopicId.trim());
    } catch (error) {
      errorHandler.log(t('p2pDebug.topicJoinFailed'), error, {
        context: 'P2PDebugPanel.handleJoinTopic',
      });
      addLog(`Failed to join topic: ${error}`);
    } finally {
      setIsLoading(false);
    }
  }, [newTopicId, joinTopic, addLog]);

  // トピック離脱
  const handleLeaveTopic = useCallback(
    async (topicId: string) => {
      setIsLoading(true);
      addLog(`Leaving topic: ${topicId}`);

      try {
        await leaveTopic(topicId);
        addLog(`Successfully left topic: ${topicId}`);
        if (selectedTopic === topicId) {
          setSelectedTopic('');
        }
      } catch (error) {
        errorHandler.log(t('p2pDebug.topicLeaveFailed'), error, {
          context: 'P2PDebugPanel.handleLeaveTopic',
        });
        addLog(`Failed to leave topic: ${error}`);
      } finally {
        setIsLoading(false);
      }
    },
    [leaveTopic, selectedTopic, addLog],
  );

  // メッセージ送信
  const handleBroadcast = useCallback(async () => {
    if (!selectedTopic || !messageContent.trim()) return;

    setIsLoading(true);
    addLog(`Broadcasting to ${selectedTopic}: ${messageContent}`);

    try {
      await broadcast(selectedTopic, messageContent.trim());
      addLog(`Message broadcast successfully`);
      setMessageContent('');
    } catch (error) {
      errorHandler.log(t('p2pDebug.messageSendFailed'), error, {
        context: 'P2PDebugPanel.handleBroadcast',
      });
      addLog(`Failed to broadcast: ${error}`);
    } finally {
      setIsLoading(false);
    }
  }, [selectedTopic, messageContent, broadcast, addLog]);

  const handleRefreshMetrics = useCallback(async () => {
    try {
      const m = await p2pApi.getMetrics();
      setMetrics(m);
      addLog(
        `Metrics updated: gossip join=${m.gossip.joins}/${m.gossip.join_details.failures} fail, routing=${formatPercent(m.mainline.routing_success_rate)} (${m.mainline.routing_successes}/${m.mainline.routing_failures}), reconnect=${m.mainline.reconnect_successes}/${m.mainline.reconnect_failures}`,
      );
    } catch (e) {
      errorHandler.log(t('p2pDebug.metricsFetchFailed'), e, {
        context: 'P2PDebugPanel.handleRefreshMetrics',
      });
      addLog(`Failed to fetch metrics: ${e}`);
    }
  }, [addLog]);

  // 開発時は定期的にメトリクスを自動更新（テスト時は無効化して安定化）
  useEffect(() => {
    // Vitest 実行時は Vite のモードが "test"
    const isTestEnv = import.meta.env.MODE === 'test';
    if (import.meta.env.PROD || isTestEnv) return;
    let disposed = false;
    (async () => {
      await handleRefreshMetrics();
    })();
    const t = setInterval(() => {
      if (!disposed) {
        handleRefreshMetrics();
      }
    }, 10000);
    return () => {
      disposed = true;
      clearInterval(t);
    };
  }, [handleRefreshMetrics]);

  // 開発環境チェック
  if (import.meta.env.PROD) {
    return null;
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center space-x-2">
          <NetworkIcon className="h-5 w-5" />
          <span>{t('p2pDebug.title')}</span>
        </CardTitle>
        <CardDescription>{t('p2pDebug.subtitle')}</CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="status" className="w-full">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="status">{t('p2pDebug.state')}</TabsTrigger>
            <TabsTrigger value="topics">{t('p2pDebug.topicsTab')}</TabsTrigger>
            <TabsTrigger value="broadcast">{t('p2pDebug.send')}</TabsTrigger>
            <TabsTrigger value="logs">{t('nostrTest.execLog')}</TabsTrigger>
          </TabsList>

          <TabsContent value="status" className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{t('p2pDebug.connectionStatus')}</span>
                <div className="flex items-center space-x-2">
                  {connectionStatus === 'connected' ? (
                    <WifiIcon className="h-4 w-4 text-green-500" />
                  ) : (
                    <WifiOffIcon className="h-4 w-4 text-red-500" />
                  )}
                  <Badge variant={connectionStatus === 'connected' ? 'default' : 'secondary'}>
                    {connectionStatus}
                  </Badge>
                </div>
              </div>

              <div className="space-y-1">
                <span className="text-sm font-medium">{t('p2pDebug.nodeId')}</span>
                <code className="text-xs font-mono bg-muted rounded px-2 py-1 block break-all">
                  {nodeId || 'N/A'}
                </code>
              </div>

              <div className="space-y-1">
                <span className="text-sm font-medium">{t('p2pDebug.nodeAddr')}</span>
                <code className="text-xs font-mono bg-muted rounded px-2 py-1 block break-all">
                  {nodeAddr || 'N/A'}
                </code>
              </div>

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{t('p2pDebug.peerCount')}</span>
                <Badge variant="outline">{peers.length}</Badge>
              </div>

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{t('p2pDebug.topicCount')}</span>
                <Badge variant="outline">{activeTopics.length}</Badge>
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">{t('p2pDebug.gossipMetrics')}</span>
                  <Button variant="secondary" size="sm" onClick={handleRefreshMetrics}>
                    {t('p2pDebug.metricsRefresh')}
                  </Button>
                </div>
                {metrics ? (
                  <div className="space-y-3">
                    <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
                      {renderMetricCard(
                        t('p2pDebug.joins'),
                        metrics.gossip.joins,
                        metrics.gossip.join_details,
                      )}
                      {renderMetricCard(
                        t('p2pDebug.leaves'),
                        metrics.gossip.leaves,
                        metrics.gossip.leave_details,
                      )}
                      {renderMetricCard(
                        t('p2pDebug.broadcasts'),
                        metrics.gossip.broadcasts_sent,
                        metrics.gossip.broadcast_details,
                      )}
                      {renderMetricCard(
                        t('p2pDebug.received'),
                        metrics.gossip.messages_received,
                        metrics.gossip.receive_details,
                      )}
                    </div>
                    <div className="rounded-md border p-3 space-y-2 text-xs sm:text-sm">
                      <div className="flex items-center justify-between">
                        <span className="text-sm font-medium">{t('p2pDebug.mainlineDht')}</span>
                        <Badge variant="outline">
                          {t('p2pDebug.peersLabel')} {metrics.mainline.connected_peers}
                        </Badge>
                      </div>
                      <div className="grid gap-2 sm:grid-cols-2">
                        <div className="space-y-0.5">
                          <span className="text-muted-foreground">
                            {t('p2pDebug.connectionAttempts')}
                          </span>
                          <span>
                            {metrics.mainline.connection_attempts}（{t('p2pDebug.success')}
                            {metrics.mainline.connection_successes} / {t('p2pDebug.failure')}
                            {metrics.mainline.connection_failures}）
                          </span>
                          <span className="text-muted-foreground">
                            {t('p2pDebug.lastSuccess')}:{' '}
                            {formatTimestamp(metrics.mainline.connection_last_success_ms)}
                          </span>
                          <span className="text-muted-foreground">
                            {t('p2pDebug.lastFailure')}:{' '}
                            {formatTimestamp(metrics.mainline.connection_last_failure_ms)}
                          </span>
                        </div>
                        <div className="space-y-0.5">
                          <span className="text-muted-foreground">
                            {t('p2pDebug.routingSuccessRate')}
                          </span>
                          <span>
                            {formatPercent(metrics.mainline.routing_success_rate)}（
                            {t('p2pDebug.success')}
                            {metrics.mainline.routing_successes} / {t('p2pDebug.failure')}
                            {metrics.mainline.routing_failures}）
                          </span>
                          <span className="text-muted-foreground">
                            {t('p2pDebug.lastSuccess')}:{' '}
                            {formatTimestamp(metrics.mainline.routing_last_success_ms)}
                          </span>
                          <span className="text-muted-foreground">
                            {t('p2pDebug.lastFailure')}:{' '}
                            {formatTimestamp(metrics.mainline.routing_last_failure_ms)}
                          </span>
                        </div>
                        <div className="space-y-0.5">
                          <span className="text-muted-foreground">{t('p2pDebug.reconnect')}</span>
                          <span>
                            {metrics.mainline.reconnect_attempts}（{t('p2pDebug.success')}
                            {metrics.mainline.reconnect_successes} / {t('p2pDebug.failure')}
                            {metrics.mainline.reconnect_failures}）
                          </span>
                        </div>
                        <div className="space-y-0.5">
                          <span className="text-muted-foreground">
                            {t('p2pDebug.lastReconnect')}
                          </span>
                          <span>
                            {t('p2pDebug.success')}:{' '}
                            {formatTimestamp(metrics.mainline.last_reconnect_success_ms)}
                          </span>
                          <span>
                            {t('p2pDebug.failure')}:{' '}
                            {formatTimestamp(metrics.mainline.last_reconnect_failure_ms)}
                          </span>
                        </div>
                      </div>
                      <Separator className="my-2" />
                      <div className="space-y-0.5 text-muted-foreground">
                        <span className="text-muted-foreground">
                          {t('p2pDebug.bootstrapStatus')}
                        </span>
                        <span>
                          {t('p2pDebug.bootstrapEnv')} {metrics.mainline.bootstrap.env_uses} /{' '}
                          {t('p2pDebug.bootstrapUser')} {metrics.mainline.bootstrap.user_uses} /{' '}
                          {t('p2pDebug.bootstrapBundle')} {metrics.mainline.bootstrap.bundle_uses} /{' '}
                          {t('p2pDebug.bootstrapFallback')}{' '}
                          {metrics.mainline.bootstrap.fallback_uses}
                        </span>
                        <span>
                          {t('p2pDebug.lastSource')}:{' '}
                          {bootstrapSourceLabel(metrics.mainline.bootstrap.last_source)}
                        </span>
                        <span>
                          {t('p2pDebug.appliedAt')}:{' '}
                          {formatTimestamp(metrics.mainline.bootstrap.last_applied_ms)}
                        </span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground">{t('p2pDebug.metricsNotFetched')}</p>
                )}
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">
                    {t('p2pDebug.nostrSubscriptionStatus')}
                  </span>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={refreshSubscriptions}
                    disabled={isSubscriptionLoading}
                  >
                    {isSubscriptionLoading ? t('p2pDebug.refreshing') : t('p2pDebug.refresh')}
                  </Button>
                </div>
                {subscriptionError && (
                  <p className="text-xs text-destructive">{subscriptionError}</p>
                )}
                <div className="grid gap-2">
                  {subscriptions.length === 0 ? (
                    <p className="text-xs text-muted-foreground">
                      {t('p2pDebug.noSubscriptionInfo')}
                    </p>
                  ) : (
                    subscriptions.map((subscription) => (
                      <div
                        key={`${subscription.targetType}:${subscription.target}`}
                        className="rounded-md border p-2 space-y-1 text-xs"
                      >
                        <div className="flex items-center justify-between">
                          <span>
                            {subscription.targetType === 'topic'
                              ? `#${subscription.target}`
                              : subscription.target}
                          </span>
                          <Badge variant="outline">{subscription.status}</Badge>
                        </div>
                        <div className="grid gap-0.5 text-[10px] text-muted-foreground">
                          <span>
                            {t('p2pDebug.lastSynced')}: {formatTimestamp(subscription.lastSyncedAt)}
                          </span>
                          <span>
                            {t('p2pDebug.lastAttempt')}:{' '}
                            {formatTimestamp(subscription.lastAttemptAt)}
                          </span>
                          {subscription.failureCount > 0 && (
                            <span>
                              {t('p2pDebug.failureCount')}: {subscription.failureCount}
                            </span>
                          )}
                          {subscription.errorMessage && (
                            <span className="text-destructive">
                              {t('p2pDebug.errorLabel')}: {subscription.errorMessage}
                            </span>
                          )}
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>

              {error && (
                <div className="bg-red-50 dark:bg-red-950 rounded p-3 space-y-2">
                  <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
                  <Button variant="ghost" size="sm" onClick={clearError}>
                    {t('p2pDebug.clearError')}
                  </Button>
                </div>
              )}
            </div>
          </TabsContent>

          {/* トピックタブ */}
          <TabsContent value="topics" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="topic-id">{t('p2pDebug.joinNewTopic')}</Label>
              <div className="flex space-x-2">
                <Input
                  id="topic-id"
                  placeholder={t('p2pDebug.topicIdPlaceholder')}
                  value={newTopicId}
                  onChange={(e) => setNewTopicId(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleJoinTopic()}
                />
                <Button onClick={handleJoinTopic} disabled={!newTopicId.trim() || isLoading}>
                  {t('p2pDebug.join')}
                </Button>
              </div>
            </div>

            <Separator />

            <div className="space-y-2">
              <h4 className="text-sm font-medium">{t('p2pDebug.joinedTopics')}</h4>
              <ScrollArea className="h-48 w-full rounded-md border">
                <div className="p-2 space-y-2">
                  {activeTopics.length === 0 ? (
                    <p className="text-sm text-muted-foreground text-center py-4">
                      {t('p2pDebug.noJoinedTopics')}
                    </p>
                  ) : (
                    activeTopics.map((topic) => (
                      <div
                        key={topic.topic_id}
                        className="flex items-center justify-between p-2 rounded hover:bg-muted/50"
                      >
                        <div className="space-y-1">
                          <code className="text-xs font-mono">{topic.topic_id}</code>
                          <div className="flex items-center space-x-2 text-xs text-muted-foreground">
                            <span>
                              {t('p2pDebug.peerLabel')}: {topic.peer_count}
                            </span>
                            <span>•</span>
                            <span>
                              {t('p2pDebug.messageCount')}: {topic.message_count}
                            </span>
                          </div>
                        </div>
                        <div className="flex items-center space-x-2">
                          <Button
                            variant={selectedTopic === topic.topic_id ? 'default' : 'ghost'}
                            size="sm"
                            onClick={() => setSelectedTopic(topic.topic_id)}
                          >
                            {t('p2pDebug.select')}
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleLeaveTopic(topic.topic_id)}
                            disabled={isLoading}
                          >
                            <TrashIcon className="h-4 w-4" />
                          </Button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </ScrollArea>
            </div>
          </TabsContent>

          {/* 送信タブ */}
          <TabsContent value="broadcast" className="space-y-4">
            {selectedTopic ? (
              <>
                <div className="space-y-2">
                  <Label>{t('p2pDebug.sendToTopic')}</Label>
                  <code className="text-sm font-mono bg-muted rounded px-2 py-1 block">
                    {selectedTopic}
                  </code>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="message">{t('p2pDebug.message')}</Label>
                  <Input
                    id="message"
                    placeholder={t('p2pDebug.messagePlaceholder')}
                    value={messageContent}
                    onChange={(e) => setMessageContent(e.target.value)}
                    onKeyPress={(e) => e.key === 'Enter' && handleBroadcast()}
                  />
                </div>
                <Button
                  onClick={handleBroadcast}
                  disabled={!messageContent.trim() || isLoading}
                  className="w-full"
                >
                  <SendIcon className="mr-2 h-4 w-4" />
                  {t('p2pDebug.broadcast')}
                </Button>
              </>
            ) : (
              <div className="text-center py-8">
                <p className="text-sm text-muted-foreground">{t('p2pDebug.selectTopicFirst')}</p>
              </div>
            )}
          </TabsContent>

          {/* ログタブ */}
          <TabsContent value="logs" className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-sm font-medium">{t('p2pDebug.debugLog')}</h4>
              <Button variant="ghost" size="sm" onClick={() => setLogs([])}>
                {t('p2pDebug.clear')}
              </Button>
            </div>
            <ScrollArea className="h-64 w-full rounded-md border">
              <div className="p-2 space-y-1">
                {logs.length === 0 ? (
                  <p className="text-xs text-muted-foreground text-center py-4">
                    {t('p2pDebug.noLogs')}
                  </p>
                ) : (
                  logs.map((log, index) => (
                    <pre key={index} className="text-xs font-mono text-muted-foreground">
                      {log}
                    </pre>
                  ))
                )}
              </div>
            </ScrollArea>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}
