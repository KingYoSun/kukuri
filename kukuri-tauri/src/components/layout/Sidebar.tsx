import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Hash, Plus, TrendingUp, Users, List, Search, MessageSquare } from 'lucide-react';
import { useTopicStore, useUIStore } from '@/stores';
import { useP2P } from '@/hooks/useP2P';
import { cn } from '@/lib/utils';
import { useNavigate } from '@tanstack/react-router';
import { RelayStatus } from '@/components/RelayStatus';
import { P2PStatus } from '@/components/P2PStatus';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useMemo } from 'react';

const categories = [
  { name: 'トピック一覧', icon: List, path: '/topics' },
  { name: '検索', icon: Search, path: '/search' },
  { name: 'トレンド', icon: TrendingUp },
  { name: 'フォロー中', icon: Users },
];

export function Sidebar() {
  const { topics, joinedTopics, currentTopic, setCurrentTopic, topicUnreadCounts } =
    useTopicStore();
  const { sidebarOpen } = useUIStore();
  const navigate = useNavigate();

  const { getTopicMessages } = useP2P();

  // 参加中トピックを最終活動時刻でソート
  const joinedTopicsList = useMemo(() => {
    const topicsList = joinedTopics
      .map((id) => {
        const topic = topics.get(id);
        if (!topic) return null;

        // P2Pメッセージから最終活動時刻を取得
        const messages = getTopicMessages(id);
        const lastMessageTime =
          messages.length > 0
            ? Math.max(...messages.map((m) => m.timestamp))
            : topic.lastActive || 0;

        const unreadCount = topicUnreadCounts.get(topic.id) ?? 0;

        return {
          ...topic,
          lastActive: lastMessageTime,
          unreadCount,
        };
      })
      .filter(Boolean) as NonNullable<ReturnType<typeof topics.get> & { unreadCount: number }>[];

    // 最終活動時刻の新しい順にソート
    return topicsList.sort((a, b) => {
      const aTime = a.lastActive || 0;
      const bTime = b.lastActive || 0;
      return bTime - aTime;
    });
  }, [joinedTopics, topics, topicUnreadCounts, getTopicMessages]);

  const handleTopicClick = (topicId: string) => {
    const topic = topics.get(topicId);
    if (topic) {
      setCurrentTopic(topic);
      navigate({ to: '/' }); // ホーム（タイムライン）に遷移
    }
  };

  return (
    <aside
      role="complementary"
      data-testid="sidebar"
      className={cn(
        'border-r bg-background transition-all duration-300 overflow-hidden',
        sidebarOpen ? 'w-64' : 'w-0',
      )}
    >
      <div className="flex flex-col h-full w-64">
        <div className="p-4">
          <Button className="w-full" variant="default">
            <Plus className="mr-2 h-4 w-4" />
            新規投稿
          </Button>
        </div>

        <Separator />

        <div className="p-4">
          <h3 className="mb-2 text-sm font-semibold text-muted-foreground">カテゴリー</h3>
          <div className="space-y-1">
            {categories.map((category) => (
              <Button
                key={category.name}
                variant="ghost"
                className="w-full justify-start"
                data-testid={`category-${category.name.toLowerCase().replace(/\s+/g, '-')}`}
                onClick={() => category.path && navigate({ to: category.path })}
              >
                <category.icon className="mr-2 h-4 w-4" />
                {category.name}
              </Button>
            ))}
          </div>
        </div>

        <Separator />

        <ScrollArea className="flex-1">
          <div className="p-4" data-testid="topics-list">
            <h3 className="mb-2 text-sm font-semibold text-muted-foreground">参加中のトピック</h3>
            <div className="space-y-1">
              {joinedTopicsList.length === 0 ? (
                <p className="text-sm text-muted-foreground">参加中のトピックはありません</p>
              ) : (
                joinedTopicsList.map((topic) => {
                  const lastActiveText = topic.lastActive
                    ? formatDistanceToNow(new Date(topic.lastActive * 1000), {
                        addSuffix: false,
                        locale: ja,
                      })
                    : '未投稿';

                  return (
                    <Button
                      key={topic.id}
                      variant={currentTopic?.id === topic.id ? 'secondary' : 'ghost'}
                      className="w-full justify-start p-2 h-auto"
                      data-testid={`topic-${topic.id}`}
                      onClick={() => handleTopicClick(topic.id)}
                    >
                      <Hash className="mr-2 h-4 w-4 flex-shrink-0" />
                      <div className="flex-1 text-left min-w-0">
                        <div className="font-medium truncate">{topic.name}</div>
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <MessageSquare className="h-3 w-3" />
                          <span>{topic.postCount}</span>
                          <span className="mx-1">·</span>
                          <span className="truncate">{lastActiveText}</span>
                        </div>
                      </div>
                      {topic.unreadCount > 0 && (
                        <Badge variant="default" className="ml-2 h-5 px-1.5 text-xs">
                          {topic.unreadCount}
                        </Badge>
                      )}
                    </Button>
                  );
                })
              )}
            </div>
          </div>
        </ScrollArea>

        <Separator />

        <div className="p-4 space-y-4">
          <RelayStatus />
          <P2PStatus />
        </div>
      </div>
    </aside>
  );
}
