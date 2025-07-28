import { createFileRoute } from '@tanstack/react-router';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Plus, Search } from 'lucide-react';
import { useState } from 'react';
import { TopicCard } from '@/components/topics/TopicCard';
import { useTopics } from '@/hooks';
import { Loader2 } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';

export const Route = createFileRoute('/topics')({
  component: TopicsPage,
});

export function TopicsPage() {
  const [searchQuery, setSearchQuery] = useState('');
  const { data: topics, isLoading, error } = useTopics();

  // 検索フィルター
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
          <Loader2 data-testid="loading-spinner" className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="container mx-auto px-4 py-8">
        <Alert variant="destructive">
          <AlertDescription>
            トピックの読み込みに失敗しました。しばらくしてから再度お試しください。
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold mb-4">トピック</h1>

        {/* 検索とアクション */}
        <div className="flex gap-4 mb-6">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="トピックを検索..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
          <Button>
            <Plus className="h-4 w-4 mr-2" />
            新規トピック
          </Button>
        </div>

        {/* トピック一覧 */}
        {filteredTopics && filteredTopics.length > 0 ? (
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            {filteredTopics.map((topic) => (
              <TopicCard key={topic.id} topic={topic} />
            ))}
          </div>
        ) : (
          <Card>
            <CardContent className="text-center py-16">
              <p className="text-muted-foreground">
                {searchQuery
                  ? '検索条件に一致するトピックが見つかりませんでした。'
                  : 'トピックがまだありません。新しいトピックを作成してください。'}
              </p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
