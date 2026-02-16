import { useTranslation } from 'react-i18next';
import i18n from '@/i18n';
import { useState, useEffect } from 'react';
import { useAuthStore } from '@/stores/authStore';
import * as nostrApi from '@/lib/api/nostr';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';
import { listen } from '@tauri-apps/api/event';
import { errorHandler } from '@/lib/errorHandler';

interface NostrEventPayload {
  id: string;
  author: string;
  content: string;
  created_at: number;
  kind: number;
  tags: string[][];
}

export function NostrTestPanel() {
  const { t } = useTranslation();
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
      addResult(`📨 ${i18n.t('nostrTest.eventReceived')}: ${event.payload.id}`);
      setReceivedEvents((prev) => [event.payload, ...prev.slice(0, 19)]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isAuthenticated]);

  const handleTestTextNote = async () => {
    if (!testContent.trim()) {
      toast.error(t('nostrTest.enterText'));
      return;
    }

    setIsLoading(true);
    try {
      const eventId = await nostrApi.publishTextNote(testContent);
      addResult(`✅ ${t('nostrTest.logTextNoteOk')}: ${eventId}`);
      toast.success(t('nostrTest.textNoteSent'));
      setTestContent('');
    } catch (error) {
      const message = error instanceof Error ? error.message : t('nostrTest.unknownError');
      errorHandler.log(t('nostrTest.textNoteSendFailed'), error, {
        context: 'NostrTestPanel.handleTestTextNote',
      });
      addResult(`❌ ${t('nostrTest.logTextNoteFail')}: ${message}`);
      toast.error(t('nostrTest.sendFailed'));
    } finally {
      setIsLoading(false);
    }
  };

  const handleTestTopicPost = async () => {
    if (!testContent.trim()) {
      toast.error(t('nostrTest.enterText'));
      return;
    }

    setIsLoading(true);
    try {
      const eventId = await nostrApi.publishTopicPost(topicId, testContent);
      addResult(`✅ ${t('nostrTest.logTopicPostOk')} (${topicId}): ${eventId}`);
      toast.success(t('nostrTest.topicPostSent'));
      setTestContent('');
    } catch (error) {
      const message = error instanceof Error ? error.message : t('nostrTest.unknownError');
      errorHandler.log(t('nostrTest.topicPostSendFailed'), error, {
        context: 'NostrTestPanel.handleTestTopicPost',
      });
      addResult(`❌ ${t('nostrTest.logTopicPostFail')}: ${message}`);
      toast.error(t('nostrTest.sendFailed'));
    } finally {
      setIsLoading(false);
    }
  };

  const handleSubscribeTopic = async () => {
    setIsLoading(true);
    try {
      await nostrApi.subscribeToTopic(topicId);
      addResult(`✅ ${t('nostrTest.logSubscribeOk')}: ${topicId}`);
      toast.success(t('nostrTest.topicSubscribed'));
    } catch (error) {
      const message = error instanceof Error ? error.message : t('nostrTest.unknownError');
      errorHandler.log(t('nostrTest.topicSubscribeFailed'), error, {
        context: 'NostrTestPanel.handleSubscribeTopic',
      });
      addResult(`❌ ${t('nostrTest.logSubscribeFail')}: ${message}`);
      toast.error(t('nostrTest.subscribeFailed'));
    } finally {
      setIsLoading(false);
    }
  };

  const handleTestReaction = async () => {
    const testEventId = prompt(t('nostrTest.reactionPrompt'));
    if (!testEventId) return;

    setIsLoading(true);
    try {
      const reactionId = await nostrApi.sendReaction(testEventId, '+');
      addResult(`✅ ${t('nostrTest.logReactionOk')}: ${reactionId}`);
      toast.success(t('nostrTest.reactionSent'));
    } catch (error) {
      const message = error instanceof Error ? error.message : t('nostrTest.unknownError');
      errorHandler.log(t('nostrTest.reactionSendFailed'), error, {
        context: 'NostrTestPanel.handleTestReaction',
      });
      addResult(`❌ ${t('nostrTest.logReactionFail')}: ${message}`);
      toast.error(t('nostrTest.sendFailed'));
    } finally {
      setIsLoading(false);
    }
  };

  if (!isAuthenticated) {
    return (
      <Card>
        <CardContent className="p-6">
          <p className="text-muted-foreground">{t('nostrTest.loginRequired')}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('nostrTest.title')}</CardTitle>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="send" className="w-full">
          <TabsList>
            <TabsTrigger value="send">{t('nostrTest.sendTest')}</TabsTrigger>
            <TabsTrigger value="subscribe">{t('nostrTest.subscribeTest')}</TabsTrigger>
            <TabsTrigger value="log">{t('nostrTest.execLog')}</TabsTrigger>
            <TabsTrigger value="received">{t('nostrTest.receivedEvents')}</TabsTrigger>
          </TabsList>

          <TabsContent value="send" className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('nostrTest.testContent')}</label>
              <Input
                placeholder={t('nostrTest.testMessagePlaceholder')}
                value={testContent}
                onChange={(e) => setTestContent(e.target.value)}
                disabled={isLoading}
              />
            </div>

            <div className="space-y-2">
              <label className="text-sm font-medium">{t('nostrTest.topicIdLabel')}</label>
              <Input
                placeholder={t('nostrTest.topicIdPlaceholder')}
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
                {t('nostrTest.textNoteSend')}
              </Button>
              <Button
                onClick={handleTestTopicPost}
                disabled={isLoading || !testContent.trim()}
                size="sm"
              >
                {t('nostrTest.topicPostSend')}
              </Button>
              <Button onClick={handleTestReaction} disabled={isLoading} size="sm" variant="outline">
                {t('nostrTest.reactionSend')}
              </Button>
            </div>
          </TabsContent>

          <TabsContent value="subscribe" className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('nostrTest.subscribeTopicIdLabel')}</label>
              <Input
                placeholder={t('nostrTest.topicIdPlaceholder')}
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
              {t('nostrTest.subscribeTopic')}
            </Button>
          </TabsContent>

          <TabsContent value="log">
            <div className="space-y-2">
              <div className="text-sm font-medium mb-2">{t('nostrTest.execResultLog')}</div>
              <div className="bg-muted p-3 rounded-md h-64 overflow-y-auto font-mono text-xs">
                {results.length === 0 ? (
                  <p className="text-muted-foreground">{t('nostrTest.noResultsYet')}</p>
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
                {t('nostrTest.receivedEventsCount', { count: receivedEvents.length })}
              </div>
              <div className="bg-muted p-3 rounded-md h-64 overflow-y-auto">
                {receivedEvents.length === 0 ? (
                  <p className="text-muted-foreground text-sm">{t('nostrTest.noEventsYet')}</p>
                ) : (
                  <div className="space-y-3">
                    {receivedEvents.map((event, index) => (
                      <div key={index} className="bg-background p-3 rounded border text-xs">
                        <div className="font-mono mb-1">
                          <span className="text-muted-foreground">ID:</span> {event.id.slice(0, 16)}
                          ...
                        </div>
                        <div>
                          <span className="text-muted-foreground">{t('nostrTest.author')}:</span>{' '}
                          {event.author.slice(0, 16)}...
                        </div>
                        <div>
                          <span className="text-muted-foreground">{t('nostrTest.kind')}:</span>{' '}
                          {event.kind}
                          {event.kind === 1 && ' (TextNote)'}
                          {event.kind === 7 && ' (Reaction)'}
                        </div>
                        <div className="mt-1">
                          <span className="text-muted-foreground">{t('nostrTest.content')}:</span>{' '}
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
