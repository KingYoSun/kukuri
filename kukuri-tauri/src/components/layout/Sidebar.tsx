import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Hash, Plus, TrendingUp, Users, List, Search, MessageSquare, Settings } from 'lucide-react';
import { useTopicStore, useUIStore, useComposerStore, type SidebarCategory } from '@/stores';
import { useP2P } from '@/hooks/useP2P';
import { cn } from '@/lib/utils';
import { useNavigate, useLocation } from '@tanstack/react-router';
import { RelayStatus } from '@/components/RelayStatus';
import { P2PStatus } from '@/components/P2PStatus';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { prefetchTrendingCategory, prefetchFollowingCategory } from '@/hooks/useTrendingFeeds';
import { TopicFormModal } from '@/components/topics/TopicFormModal';
import type { Topic } from '@/stores';

interface SidebarCategoryItem {
  key: SidebarCategory;
  name: string;
  icon: typeof List;
  path: string;
}

const categories: SidebarCategoryItem[] = [
  { key: 'topics', name: 'トピック一覧', icon: List, path: '/topics' },
  { key: 'search', name: '検索', icon: Search, path: '/search' },
  { key: 'trending', name: 'トレンド', icon: TrendingUp, path: '/trending' },
  { key: 'following', name: 'フォロー中', icon: Users, path: '/following' },
];

const deriveCategoryFromPath = (pathname: string): SidebarCategory | null => {
  if (pathname.startsWith('/topics')) {
    return 'topics';
  }
  if (pathname.startsWith('/search')) {
    return 'search';
  }
  if (pathname.startsWith('/trending')) {
    return 'trending';
  }
  if (pathname.startsWith('/following')) {
    return 'following';
  }
  return null;
};

export function Sidebar() {
  const { topics, joinedTopics, currentTopic, setCurrentTopic, topicUnreadCounts } =
    useTopicStore();
  const [showTopicCreationDialog, setShowTopicCreationDialog] = useState(false);
  const sidebarOpen = useUIStore((state) => state.sidebarOpen);
  const activeSidebarCategory = useUIStore((state) => state.activeSidebarCategory);
  const setActiveSidebarCategory = useUIStore((state) => state.setActiveSidebarCategory);
  const { openComposer } = useComposerStore();
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();

  const { getTopicMessages } = useP2P();

  useEffect(() => {
    const derivedCategory = deriveCategoryFromPath(location.pathname);
    setActiveSidebarCategory(derivedCategory);
  }, [location.pathname, setActiveSidebarCategory]);

  // 参加中トピックを最終活動時刻でソート
  const joinedTopicsList = useMemo(() => {
    const topicsList = joinedTopics
      .map((id) => {
        const topic = topics.get(id);
        if (!topic) return null;

        // P2Pメッセージから最終活動時刻を取得
        const messages = getTopicMessages(id);
        const messageTimestamps = messages.map((m) =>
          m.timestamp > 1_000_000_000_000
            ? Math.floor(m.timestamp / 1000)
            : Math.floor(m.timestamp),
        );
        const lastMessageTime =
          messageTimestamps.length > 0 ? Math.max(...messageTimestamps) : topic.lastActive || 0;

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
      setActiveSidebarCategory(null);
      navigate({ to: '/' }); // ホーム（タイムライン）に遷移
    }
  };

  const handleCreatePost = () => {
    if (joinedTopicsList.length === 0) {
      setShowTopicCreationDialog(true);
      return;
    }

    const fallbackTopicId = currentTopic?.id ?? joinedTopics[0] ?? undefined;
    openComposer({
      topicId: fallbackTopicId,
    });
  };

  const handleSidebarTopicCreated = (topic: Topic) => {
    setShowTopicCreationDialog(false);
    openComposer({ topicId: topic.id });
  };

  const handleOpenSettings = () => {
    setCurrentTopic(null);
    setActiveSidebarCategory(null);
    navigate({ to: '/settings' });
  };

  const handleCategoryClick = useCallback(
    (category: SidebarCategoryItem) => {
      setCurrentTopic(null);
      setActiveSidebarCategory(category.key);
      if (category.key === 'trending') {
        void prefetchTrendingCategory(queryClient);
      } else if (category.key === 'following') {
        void prefetchFollowingCategory(queryClient);
      }

      navigate({ to: category.path });
    },
    [navigate, queryClient, setActiveSidebarCategory, setCurrentTopic],
  );

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
          <Button className="w-full" variant="default" onClick={handleCreatePost}>
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
                key={category.key}
                variant={activeSidebarCategory === category.key ? 'secondary' : 'ghost'}
                className={cn(
                  'w-full justify-start',
                  activeSidebarCategory === category.key && 'font-semibold',
                )}
                data-testid={`category-${category.key}`}
                aria-current={activeSidebarCategory === category.key ? 'page' : undefined}
                onClick={() => handleCategoryClick(category)}
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
                        <Badge
                          variant="default"
                          className="ml-2 h-5 px-1.5 text-xs"
                          data-testid={`topic-${topic.id}-unread`}
                        >
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
          <Button
            variant="ghost"
            className="w-full justify-start"
            onClick={handleOpenSettings}
            data-testid="open-settings-button"
          >
            <Settings className="mr-2 h-4 w-4" />
            設定
          </Button>
        </div>
      </div>
      <TopicFormModal
        open={showTopicCreationDialog}
        onOpenChange={setShowTopicCreationDialog}
        mode="create-from-composer"
        autoJoin
        onCreated={handleSidebarTopicCreated}
      />
    </aside>
  );
}
