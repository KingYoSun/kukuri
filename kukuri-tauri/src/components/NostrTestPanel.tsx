import { useState, useEffect } from 'react';
import { useAuthStore } from '@/stores/authStore';
import * as nostrApi from '@/lib/api/nostr';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';
import { listen } from '@tauri-apps/api/event';

interface NostrEventPayload {
  id: string;
  author: string;
  content: string;
  created_at: number;
  kind: number;
  tags: string[][];
}

export function NostrTestPanel() {
  const { isAuthenticated } = useAuthStore();
  const [testContent, setTestContent] = useState('');
  const [topicId, setTopicId] = useState('kukuri-test');
  const [isLoading, setIsLoading] = useState(false);
  const [results, setResults] = useState<string[]>([]);
  const [receivedEvents, setReceivedEvents] = useState<NostrEventPayload[]>([]);

  const addResult = (message: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setResults((prev) => [`[${timestamp}] ${message}`, ...prev.slice(0, 9)]);
  };

  useEffect(() => {
    if (!isAuthenticated) return;

    // Nostrイベントをリッスン
    const unlisten = listen<NostrEventPayload>('nostr://event', (event) => {
      addResult(`📨 イベント受信: ${event.payload.id}`);
      setReceivedEvents((prev) => [event.payload, ...prev.slice(0, 19)]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isAuthenticated]);

  const handleTestTextNote = async () => {
    if (!testContent.trim()) {
      toast.error('テキストを入力してください');
      return;
    }

    setIsLoading(true);
    try {
      const eventId = await nostrApi.publishTextNote(testContent);
      addResult(`✅ テキストノート送信成功: ${eventId}`);
      toast.success('テキストノートを送信しました');
      setTestContent('');
    } catch (error) {
      const message = error instanceof Error ? error.message : '不明なエラー';
      addResult(`❌ テキストノート送信失敗: ${message}`);
      toast.error('送信に失敗しました');
    } finally {
      setIsLoading(false);
    }
  };

  const handleTestTopicPost = async () => {
    if (!testContent.trim()) {
      toast.error('テキストを入力してください');
      return;
    }

    setIsLoading(true);
    try {
      const eventId = await nostrApi.publishTopicPost(topicId, testContent);
      addResult(`✅ トピック投稿送信成功 (${topicId}): ${eventId}`);
      toast.success('トピック投稿を送信しました');
      setTestContent('');
    } catch (error) {
      const message = error instanceof Error ? error.message : '不明なエラー';
      addResult(`❌ トピック投稿送信失敗: ${message}`);
      toast.error('送信に失敗しました');
    } finally {
      setIsLoading(false);
    }
  };

  const handleSubscribeTopic = async () => {
    setIsLoading(true);
    try {
      await nostrApi.subscribeToTopic(topicId);
      addResult(`✅ トピック購読成功: ${topicId}`);
      toast.success('トピックを購読しました');
    } catch (error) {
      const message = error instanceof Error ? error.message : '不明なエラー';
      addResult(`❌ トピック購読失敗: ${message}`);
      toast.error('購読に失敗しました');
    } finally {
      setIsLoading(false);
    }
  };

  const handleTestReaction = async () => {
    const testEventId = prompt('リアクションを送信するイベントIDを入力してください:');
    if (!testEventId) return;

    setIsLoading(true);
    try {
      const reactionId = await nostrApi.sendReaction(testEventId, '+');
      addResult(`✅ リアクション送信成功: ${reactionId}`);
      toast.success('リアクションを送信しました');
    } catch (error) {
      const message = error instanceof Error ? error.message : '不明なエラー';
      addResult(`❌ リアクション送信失敗: ${message}`);
      toast.error('送信に失敗しました');
    } finally {
      setIsLoading(false);
    }
  };

  if (!isAuthenticated) {
    return (
      <Card>
        <CardContent className="p-6">
          <p className="text-muted-foreground">ログインしてください</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Nostrイベント送受信テスト</CardTitle>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="send" className="w-full">
          <TabsList>
            <TabsTrigger value="send">送信テスト</TabsTrigger>
            <TabsTrigger value="subscribe">購読テスト</TabsTrigger>
            <TabsTrigger value="log">実行ログ</TabsTrigger>
            <TabsTrigger value="received">受信イベント</TabsTrigger>
          </TabsList>

          <TabsContent value="send" className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">テスト内容</label>
              <Input
                placeholder="テストメッセージを入力"
                value={testContent}
                onChange={(e) => setTestContent(e.target.value)}
                disabled={isLoading}
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">トピックID（トピック投稿用）</label>
              <Input
                placeholder="トピックID"
                value={topicId}
                onChange={(e) => setTopicId(e.target.value)}
                disabled={isLoading}
              />
            </div>

            <div className="flex flex-wrap gap-2">
              <Button
                onClick={handleTestTextNote}
                disabled={isLoading || !testContent.trim()}
                size="sm"
              >
                テキストノート送信
              </Button>
              <Button
                onClick={handleTestTopicPost}
                disabled={isLoading || !testContent.trim()}
                size="sm"
              >
                トピック投稿送信
              </Button>
              <Button onClick={handleTestReaction} disabled={isLoading} size="sm" variant="outline">
                リアクション送信
              </Button>
            </div>
          </TabsContent>

          <TabsContent value="subscribe" className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">購読するトピックID</label>
              <Input
                placeholder="トピックID"
                value={topicId}
                onChange={(e) => setTopicId(e.target.value)}
                disabled={isLoading}
              />
            </div>

            <Button
              onClick={handleSubscribeTopic}
              disabled={isLoading || !topicId.trim()}
              size="sm"
            >
              トピックを購読
            </Button>
          </TabsContent>

          <TabsContent value="log">
            <div className="space-y-2">
              <div className="text-sm font-medium mb-2">実行結果ログ</div>
              <div className="bg-muted p-3 rounded-md h-64 overflow-y-auto font-mono text-xs">
                {results.length === 0 ? (
                  <p className="text-muted-foreground">まだ実行結果がありません</p>
                ) : (
                  results.map((result, index) => (
                    <div key={index} className="mb-1">
                      {result}
                    </div>
                  ))
                )}
              </div>
            </div>
          </TabsContent>

          <TabsContent value="received">
            <div className="space-y-2">
              <div className="text-sm font-medium mb-2">
                受信イベント ({receivedEvents.length}件)
              </div>
              <div className="bg-muted p-3 rounded-md h-64 overflow-y-auto">
                {receivedEvents.length === 0 ? (
                  <p className="text-muted-foreground text-sm">まだイベントを受信していません</p>
                ) : (
                  <div className="space-y-3">
                    {receivedEvents.map((event, index) => (
                      <div key={index} className="bg-background p-3 rounded border text-xs">
                        <div className="font-mono mb-1">
                          <span className="text-muted-foreground">ID:</span> {event.id.slice(0, 16)}
                          ...
                        </div>
                        <div>
                          <span className="text-muted-foreground">著者:</span>{' '}
                          {event.author.slice(0, 16)}...
                        </div>
                        <div>
                          <span className="text-muted-foreground">種類:</span> {event.kind}
                          {event.kind === 1 && ' (TextNote)'}
                          {event.kind === 7 && ' (Reaction)'}
                        </div>
                        <div className="mt-1">
                          <span className="text-muted-foreground">内容:</span>{' '}
                          {event.content.slice(0, 100)}
                          {event.content.length > 100 && '...'}
                        </div>
                        <div className="text-muted-foreground text-xs mt-1">
                          {new Date(event.created_at * 1000).toLocaleString()}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </TabsContent>
        </Tabs>
      </CardContent>
    </Card>
  );
}
