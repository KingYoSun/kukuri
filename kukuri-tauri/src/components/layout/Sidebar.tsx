import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Hash, Plus, TrendingUp, Users, List, Search, MessageSquare, Settings } from 'lucide-react';
import { useTopicStore, useUIStore, useComposerStore, type SidebarCategory } from '@/stores';
import { useP2P } from '@/hooks/useP2P';
import { cn } from '@/lib/utils';
import { useNavigate, useLocation } from '@tanstack/react-router';
import { formatDistanceToNow } from 'date-fns';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { prefetchTrendingCategory, prefetchFollowingCategory } from '@/hooks/useTrendingFeeds';
import { TopicFormModal } from '@/components/topics/TopicFormModal';
import type { Topic } from '@/stores';
import { getDateFnsLocale } from '@/i18n';

interface SidebarCategoryItem {
  key: SidebarCategory;
  nameKey: string;
  icon: typeof List;
  path: string;
}

const categoryKeys: SidebarCategoryItem[] = [
  { key: 'topics', nameKey: 'nav.topics', icon: List, path: '/topics' },
  { key: 'search', nameKey: 'nav.search', icon: Search, path: '/search' },
  { key: 'trending', nameKey: 'nav.trending', icon: TrendingUp, path: '/trending' },
  { key: 'following', nameKey: 'nav.following', icon: Users, path: '/following' },
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

const SIDEBAR_MIN_WIDTH_PX = 256;
const SIDEBAR_MAX_WIDTH_PX = 420;
const DESKTOP_MAIN_CONTENT_MIN_WIDTH_PX = 480;
const MOBILE_BREAKPOINT_PX = 768;
const VIEWPORT_MARGIN_PX = 24;

const clamp = (value: number, min: number, max: number): number => {
  if (max < min) {
    return min;
  }
  return Math.min(Math.max(value, min), max);
};

const getResponsiveSidebarRange = (viewportWidth: number) => {
  const safeViewportWidth = Number.isFinite(viewportWidth) && viewportWidth > 0 ? viewportWidth : 0;
  const preferredMaxWidth =
    safeViewportWidth < MOBILE_BREAKPOINT_PX
      ? safeViewportWidth - VIEWPORT_MARGIN_PX
      : safeViewportWidth - DESKTOP_MAIN_CONTENT_MIN_WIDTH_PX;
  const maxWidth = clamp(preferredMaxWidth, 0, SIDEBAR_MAX_WIDTH_PX);
  const minCandidateWidth = Math.min(
    SIDEBAR_MIN_WIDTH_PX,
    Math.max(0, safeViewportWidth - VIEWPORT_MARGIN_PX * 2),
  );
  const minWidth = Math.min(minCandidateWidth, maxWidth);

  return { minWidth, maxWidth };
};

export function Sidebar() {
  const { t, i18n } = useTranslation();
  const { topics, joinedTopics, currentTopic, setCurrentTopic, topicUnreadCounts } =
    useTopicStore();
  const [showTopicCreationDialog, setShowTopicCreationDialog] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR_MIN_WIDTH_PX);
  const sidebarContentRef = useRef<HTMLDivElement>(null);
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

  const recalculateSidebarWidth = useCallback(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const { minWidth, maxWidth } = getResponsiveSidebarRange(window.innerWidth);
    const contentElement = sidebarContentRef.current;

    if (!contentElement) {
      setSidebarWidth((prev) => clamp(prev, minWidth, maxWidth));
      return;
    }

    const requiredWidth = Math.ceil(contentElement.scrollWidth);
    const nextWidth = clamp(requiredWidth, minWidth, maxWidth);
    setSidebarWidth((prev) => (prev === nextWidth ? prev : nextWidth));
  }, []);

  useEffect(() => {
    if (!sidebarOpen) {
      return;
    }

    recalculateSidebarWidth();

    const contentElement = sidebarContentRef.current;
    const resizeObserver =
      contentElement === null ? null : new ResizeObserver(() => recalculateSidebarWidth());
    if (contentElement) {
      resizeObserver?.observe(contentElement);
    }

    window.addEventListener('resize', recalculateSidebarWidth);

    return () => {
      resizeObserver?.disconnect();
      window.removeEventListener('resize', recalculateSidebarWidth);
    };
  }, [i18n.language, joinedTopicsList, recalculateSidebarWidth, sidebarOpen]);

  const currentSidebarWidth = sidebarOpen ? sidebarWidth : 0;

  return (
    <aside
      role="complementary"
      data-testid="sidebar"
      className={cn(
        'bg-background transition-[width] duration-300 overflow-hidden h-full shrink-0',
        sidebarOpen ? 'border-r' : 'border-r-0',
      )}
      style={{ width: `${currentSidebarWidth}px` }}
    >
      <div ref={sidebarContentRef} className="flex h-full w-full min-h-0 flex-col">
        <ScrollArea className="h-full w-full">
          <div className="flex min-h-full flex-col">
            <div className="p-4">
              <Button className="w-full" variant="default" onClick={handleCreatePost}>
                <Plus className="mr-2 h-4 w-4" />
                {t('nav.newPost')}
              </Button>
            </div>

            <Separator />

            <div className="p-4">
              <h3 className="mb-2 text-sm font-semibold text-muted-foreground">
                {t('nav.category')}
              </h3>
              <div className="space-y-1">
                {categoryKeys.map((category) => (
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
                    {t(category.nameKey)}
                  </Button>
                ))}
              </div>
            </div>

            <Separator />

            <div className="p-4" data-testid="topics-list">
              <h3 className="mb-2 text-sm font-semibold text-muted-foreground">
                {t('nav.joinedTopics')}
              </h3>
              <div className="space-y-1">
                {joinedTopicsList.length === 0 ? (
                  <p className="text-sm text-muted-foreground">{t('nav.noJoinedTopics')}</p>
                ) : (
                  joinedTopicsList.map((topic) => {
                    const lastActiveText = topic.lastActive
                      ? formatDistanceToNow(new Date(topic.lastActive * 1000), {
                          addSuffix: false,
                          locale: getDateFnsLocale(),
                        })
                      : t('nav.noPostsYet');

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

            <Separator />

            <div className="p-4">
              <Button
                variant="ghost"
                className="w-full justify-start"
                onClick={handleOpenSettings}
                data-testid="open-settings-button"
              >
                <Settings className="mr-2 h-4 w-4" />
                {t('nav.settings')}
              </Button>
            </div>
          </div>
        </ScrollArea>
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
