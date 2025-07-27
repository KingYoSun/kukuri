import { useState, useCallback } from 'react';
import { useP2P } from '@/hooks/useP2P';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { SendIcon, NetworkIcon, TrashIcon, WifiIcon, WifiOffIcon } from 'lucide-react';

export function P2PDebugPanel() {
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

  const [newTopicId, setNewTopicId] = useState('');
  const [selectedTopic, setSelectedTopic] = useState('');
  const [messageContent, setMessageContent] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);

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
      addLog(`Failed to broadcast: ${error}`);
    } finally {
      setIsLoading(false);
    }
  }, [selectedTopic, messageContent, broadcast, addLog]);

  // 開発環境チェック
  if (import.meta.env.PROD) {
    return null;
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="flex items-center space-x-2">
          <NetworkIcon className="h-5 w-5" />
          <span>P2P Debug Panel</span>
        </CardTitle>
        <CardDescription>P2P機能のテストとデバッグ（開発環境のみ）</CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="status" className="w-full">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="status">状態</TabsTrigger>
            <TabsTrigger value="topics">トピック</TabsTrigger>
            <TabsTrigger value="broadcast">送信</TabsTrigger>
            <TabsTrigger value="logs">ログ</TabsTrigger>
          </TabsList>

          {/* 状態タブ */}
          <TabsContent value="status" className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">接続状態</span>
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
                <span className="text-sm font-medium">ノードID</span>
                <code className="text-xs font-mono bg-muted rounded px-2 py-1 block break-all">
                  {nodeId || 'N/A'}
                </code>
              </div>

              <div className="space-y-1">
                <span className="text-sm font-medium">ノードアドレス</span>
                <code className="text-xs font-mono bg-muted rounded px-2 py-1 block break-all">
                  {nodeAddr || 'N/A'}
                </code>
              </div>

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">接続ピア数</span>
                <Badge variant="outline">{peers.length}</Badge>
              </div>

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">参加トピック数</span>
                <Badge variant="outline">{activeTopics.length}</Badge>
              </div>

              {error && (
                <div className="bg-red-50 dark:bg-red-950 rounded p-3 space-y-2">
                  <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
                  <Button variant="ghost" size="sm" onClick={clearError}>
                    エラーをクリア
                  </Button>
                </div>
              )}
            </div>
          </TabsContent>

          {/* トピックタブ */}
          <TabsContent value="topics" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="topic-id">新しいトピックに参加</Label>
              <div className="flex space-x-2">
                <Input
                  id="topic-id"
                  placeholder="トピックID (例: test-topic)"
                  value={newTopicId}
                  onChange={(e) => setNewTopicId(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleJoinTopic()}
                />
                <Button onClick={handleJoinTopic} disabled={!newTopicId.trim() || isLoading}>
                  参加
                </Button>
              </div>
            </div>

            <Separator />

            <div className="space-y-2">
              <h4 className="text-sm font-medium">参加中のトピック</h4>
              <ScrollArea className="h-48 w-full rounded-md border">
                <div className="p-2 space-y-2">
                  {activeTopics.length === 0 ? (
                    <p className="text-sm text-muted-foreground text-center py-4">
                      参加中のトピックはありません
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
                            <span>ピア: {topic.peer_count}</span>
                            <span>•</span>
                            <span>メッセージ: {topic.message_count}</span>
                          </div>
                        </div>
                        <div className="flex items-center space-x-2">
                          <Button
                            variant={selectedTopic === topic.topic_id ? 'default' : 'ghost'}
                            size="sm"
                            onClick={() => setSelectedTopic(topic.topic_id)}
                          >
                            選択
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
                  <Label>送信先トピック</Label>
                  <code className="text-sm font-mono bg-muted rounded px-2 py-1 block">
                    {selectedTopic}
                  </code>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="message">メッセージ</Label>
                  <Input
                    id="message"
                    placeholder="送信するメッセージを入力"
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
                  ブロードキャスト
                </Button>
              </>
            ) : (
              <div className="text-center py-8">
                <p className="text-sm text-muted-foreground">トピックを選択してください</p>
              </div>
            )}
          </TabsContent>

          {/* ログタブ */}
          <TabsContent value="logs" className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-sm font-medium">デバッグログ</h4>
              <Button variant="ghost" size="sm" onClick={() => setLogs([])}>
                クリア
              </Button>
            </div>
            <ScrollArea className="h-64 w-full rounded-md border">
              <div className="p-2 space-y-1">
                {logs.length === 0 ? (
                  <p className="text-xs text-muted-foreground text-center py-4">ログはありません</p>
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
