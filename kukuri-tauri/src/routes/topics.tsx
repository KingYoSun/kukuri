import { useTranslation } from 'react-i18next';
import { createFileRoute, Outlet, useLocation } from '@tanstack/react-router';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Plus, Search, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { TopicCard } from '@/components/topics/TopicCard';
import { TopicFormModal } from '@/components/topics/TopicFormModal';
import { useTopics } from '@/hooks';
import { Alert, AlertDescription } from '@/components/ui/alert';

export const Route = createFileRoute('/topics')({
  component: TopicsPage,
});

export function TopicsPage() {
  const { t } = useTranslation();
  let isDetailPage = false;
  try {
    const currentLocation = useLocation();
    isDetailPage =
      currentLocation.pathname.startsWith('/topics/') && currentLocation.pathname !== '/topics';
  } catch {
    const fallbackPath =
      typeof window !== 'undefined' && window.location?.pathname ? window.location.pathname : '';
    isDetailPage = fallbackPath.startsWith('/topics/') && fallbackPath !== '/topics';
  }
  const [searchQuery, setSearchQuery] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const { data: topics, isLoading, error } = useTopics();

  const filteredTopics = topics?.filter((topic) => {
    const query = searchQuery.toLowerCase();
    return (
      topic.name.toLowerCase().includes(query) ||
      topic.description.toLowerCase().includes(query) ||
      topic.tags.some((tag) => tag.toLowerCase().includes(query))
    );
  });

  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="flex items-center justify-center py-16">
          <Loader2
            data-testid="loading-spinner"
            className="h-8 w-8 animate-spin text-muted-foreground"
          />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="container mx-auto px-4 py-8">
        <Alert variant="destructive">
          <AlertDescription>{t('topics.loadFailed')}</AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="max-w-6xl mx-auto">
        {isDetailPage ? (
          <Outlet />
        ) : (
          <>
            <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4 mb-8">
              <h1 className="text-3xl font-bold">{t('topics.title')}</h1>
              <Button onClick={() => setShowCreateModal(true)} data-testid="open-topic-create">
                <Plus className="h-4 w-4 mr-2" />
                {t('topics.newTopic')}
              </Button>
            </div>

            <div className="mb-6">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  type="search"
                  placeholder={t('topics.searchPlaceholder')}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10"
                />
              </div>
            </div>

            {filteredTopics && filteredTopics.length > 0 ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {filteredTopics.map((topic) => (
                  <TopicCard key={topic.id} topic={topic} />
                ))}
              </div>
            ) : (
              <Card className="p-8">
                <CardContent className="text-center">
                  <p className="text-muted-foreground">
                    {searchQuery ? t('topics.noMatch') : t('topics.empty')}
                  </p>
                </CardContent>
              </Card>
            )}
          </>
        )}
      </div>

      {!isDetailPage && (
        <TopicFormModal open={showCreateModal} onOpenChange={setShowCreateModal} mode="create" />
      )}
    </div>
  );
}
