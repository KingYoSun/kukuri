import { useTranslation } from 'react-i18next';
import { useQuery } from '@tanstack/react-query';
import { TopicCard } from '@/components/topics/TopicCard';
import { Loader2 } from 'lucide-react';
import { useTopics } from '@/hooks/useTopics';
import type { Topic } from '@/stores';

interface TopicSearchResultsProps {
  query: string;
}

export function TopicSearchResults({ query }: TopicSearchResultsProps) {
  const { t } = useTranslation();
  const { data: allTopics, isLoading } = useTopics();

  // クライアントサイドでトピックを検索
  const searchResults = useQuery({
    queryKey: ['search', 'topics', query],
    queryFn: async () => {
      if (!query || !allTopics) return [];

      const searchTerm = query.toLowerCase();

      // トピック名と説明で検索
      return allTopics.filter((topic) => {
        const nameMatch = topic.name.toLowerCase().includes(searchTerm);
        const descriptionMatch = topic.description?.toLowerCase().includes(searchTerm) || false;

        return nameMatch || descriptionMatch;
      });
    },
    enabled: !!query && !!allTopics,
    staleTime: 0, // 常に最新のデータで検索
  });

  if (!query) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        {t('search.enterKeyword')}
      </div>
    );
  }

  if (isLoading || searchResults.isLoading) {
    return (
      <div className="flex justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const results = searchResults.data || [];

  if (results.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-lg font-medium">{t('search.noTopicResults')}</p>
        <p className="text-muted-foreground mt-2">{t('search.noTopicResultsDescription', { query })}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{t('search.topicsFound', { count: results.length })}</p>
      <div className="grid gap-4 sm:grid-cols-2">
        {results.map((topic) => (
          <SearchResultTopic key={topic.id} topic={topic} query={query} />
        ))}
      </div>
    </div>
  );
}

// 検索結果用のトピックカード
function SearchResultTopic({ topic }: { topic: Topic; query: string }) {
  return <TopicCard topic={topic} />;
}
