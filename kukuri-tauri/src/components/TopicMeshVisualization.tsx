import { useTranslation } from 'react-i18next';
import { useP2P } from '@/hooks/useP2P';
import type { P2PMessage } from '@/stores/p2pStore';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import {
  NetworkIcon,
  UsersIcon,
  MessageSquareIcon,
  ActivityIcon,
  CircleIcon,
  RefreshCwIcon,
} from 'lucide-react';
import { useState, useCallback, useEffect } from 'react';

interface TopicMeshVisualizationProps {
  topicId: string;
}

export function TopicMeshVisualization({ topicId }: TopicMeshVisualizationProps) {
  const { t } = useTranslation();
  const { getTopicStats, getTopicMessages, isJoinedTopic, joinTopic, leaveTopic } = useP2P();

  const [isRefreshing, setIsRefreshing] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const topicStats = getTopicStats(topicId);
  const messages = getTopicMessages(topicId);
  const isJoined = isJoinedTopic(topicId);

  // 自動更新
  useEffect(() => {
    if (autoRefresh && isJoined) {
      const interval = setInterval(() => {
        // P2Pストアが自動的に更新されるため、ここでは特に何もしない
        // 状態の変更により自動的に再レンダリングされる
      }, 5000);

      return () => clearInterval(interval);
    }
  }, [autoRefresh, isJoined]);

  const handleJoinTopic = useCallback(async () => {
    setIsRefreshing(true);
    try {
      await joinTopic(topicId);
    } finally {
      setIsRefreshing(false);
    }
  }, [topicId, joinTopic]);

  const handleLeaveTopic = useCallback(async () => {
    setIsRefreshing(true);
    try {
      await leaveTopic(topicId);
    } finally {
      setIsRefreshing(false);
    }
  }, [topicId, leaveTopic]);

  if (!isJoined) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-lg flex items-center space-x-2">
            <NetworkIcon className="h-5 w-5" />
            <span>{t('topicMesh.title')}</span>
          </CardTitle>
          <CardDescription>{t('topicMesh.description')}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-center py-8 space-y-4">
            <NetworkIcon className="h-12 w-12 text-muted-foreground mx-auto" />
            <p className="text-sm text-muted-foreground">
              {t('topicMesh.notJoined')}
            </p>
            <Button onClick={handleJoinTopic} disabled={isRefreshing}>
              {isRefreshing ? (
                <>
                  <RefreshCwIcon className="mr-2 h-4 w-4 animate-spin" />
                  {t('topicMesh.connecting')}
                </>
              ) : (
                t('topicMesh.joinNetwork')
              )}
            </Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-lg flex items-center space-x-2">
              <NetworkIcon className="h-5 w-5" />
              <span>{t('topicMesh.title')}</span>
            </CardTitle>
            <CardDescription>{t('topicMesh.description')}</CardDescription>
          </div>
          <div className="flex items-center space-x-2">
            <Button variant="ghost" size="sm" onClick={() => setAutoRefresh(!autoRefresh)}>
              <ActivityIcon
                className={`h-4 w-4 ${autoRefresh ? 'text-green-500' : 'text-muted-foreground'}`}
              />
            </Button>
            <Button variant="ghost" size="sm" onClick={handleLeaveTopic} disabled={isRefreshing}>
              {t('topicMesh.disconnect')}
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 統計情報 */}
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <div className="flex items-center space-x-2 text-sm text-muted-foreground">
              <UsersIcon className="h-4 w-4" />
              <span>{t('topicMesh.connectedPeers')}</span>
            </div>
            <p className="text-2xl font-bold">{topicStats?.peer_count || 0}</p>
          </div>
          <div className="space-y-2">
            <div className="flex items-center space-x-2 text-sm text-muted-foreground">
              <MessageSquareIcon className="h-4 w-4" />
              <span>{t('topicMesh.messageCount')}</span>
            </div>
            <p className="text-2xl font-bold">{topicStats?.message_count || 0}</p>
          </div>
        </div>

        <Separator />

        {/* 接続ピア一覧 */}
        {topicStats && topicStats.connected_peers.length > 0 && (
          <div className="space-y-2">
            <h4 className="text-sm font-medium text-muted-foreground">{t('topicMesh.connectedPeersList')}</h4>
            <ScrollArea className="h-32 w-full rounded-md border">
              <div className="p-2 space-y-2">
                {topicStats.connected_peers.map((peerId: string) => (
                  <div
                    key={peerId}
                    className="flex items-center justify-between p-2 rounded-md hover:bg-muted/50"
                  >
                    <div className="flex items-center space-x-2">
                      <CircleIcon className="h-2 w-2 fill-green-500 text-green-500" />
                      <code className="text-xs font-mono">{peerId.slice(0, 16)}...</code>
                    </div>
                    <Badge variant="outline" className="text-xs">
                      {t('topicMesh.connected')}
                    </Badge>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </div>
        )}

        {/* 最近のメッセージ */}
        {messages.length > 0 && (
          <>
            <Separator />
            <div className="space-y-2">
              <h4 className="text-sm font-medium text-muted-foreground">{t('topicMesh.recentMessages')}</h4>
              <ScrollArea className="h-48 w-full rounded-md border">
                <div className="p-2 space-y-2">
                  {messages.slice(0, 10).map((message: P2PMessage) => (
                    <div key={message.id} className="space-y-1 p-2 rounded-md hover:bg-muted/50">
                      <div className="flex items-center justify-between">
                        <code className="text-xs font-mono text-muted-foreground">
                          {message.author.slice(0, 8)}...
                        </code>
                        <span className="text-xs text-muted-foreground">
                          {new Date(message.timestamp * 1000).toLocaleTimeString()}
                        </span>
                      </div>
                      <p className="text-sm break-all">{message.content}</p>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </div>
          </>
        )}

        {/* 空状態 */}
        {(!topicStats || (topicStats.peer_count === 0 && topicStats.message_count === 0)) && (
          <div className="text-center py-8">
            <NetworkIcon className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">{t('topicMesh.noPeersConnected')}</p>
            <p className="text-xs text-muted-foreground mt-1">
              {t('topicMesh.waitingForNodes')}
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
