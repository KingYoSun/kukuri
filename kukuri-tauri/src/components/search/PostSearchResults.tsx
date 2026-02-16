import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useInfiniteQuery, useQuery, type InfiniteData } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';

import { PostCard } from '@/components/posts/PostCard';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { useDebounce } from '@/hooks/useDebounce';
import { usePosts } from '@/hooks/usePosts';
import {
  communityNodeApi,
  type CommunityNodeSearchHit,
  type CommunityNodeSearchResponse,
} from '@/lib/api/communityNode';
import { errorHandler } from '@/lib/errorHandler';
import { useTopicStore, type Post } from '@/stores';

interface PostSearchResultsProps {
  query: string;
}

const COMMUNITY_NODE_PAGE_SIZE = 5;

type NormalizedSearchHit = {
  eventId: string;
  topicId: string;
  title: string;
  summary: string;
  content: string;
  author: string;
  createdAt: number;
  tags: string[];
};

const toString = (value: unknown): string => (typeof value === 'string' ? value : '');
const toNumber = (value: unknown): number =>
  typeof value === 'number' && Number.isFinite(value) ? value : 0;
const toStringArray = (value: unknown): string[] =>
  Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];

const normalizeSearchHit = (raw: CommunityNodeSearchHit): NormalizedSearchHit => {
  const rawRecord = raw as Record<string, unknown>;
  const eventId = toString(raw.event_id ?? rawRecord.eventId);
  const topicId = toString(raw.topic_id ?? rawRecord.topicId);
  return {
    eventId,
    topicId,
    title: toString(raw.title),
    summary: toString(raw.summary),
    content: toString(raw.content),
    author: toString(raw.author),
    createdAt: toNumber(raw.created_at),
    tags: toStringArray(raw.tags),
  };
};

async function fetchCommunityNodeSearchPage(params: {
  topicId: string;
  query: string;
  cursor: string | null;
  limit: number;
}): Promise<CommunityNodeSearchResponse> {
  const request = {
    topic: params.topicId,
    q: params.query,
    limit: params.limit,
    cursor: params.cursor ?? undefined,
  };
  return await communityNodeApi.search(request);
}

export function PostSearchResults({ query }: PostSearchResultsProps) {
  const communityConfigQuery = useQuery({
    queryKey: ['community-node', 'config'],
    queryFn: () => communityNodeApi.getConfig(),
    staleTime: 1000 * 60 * 5,
  });
  const searchNodes =
    communityConfigQuery.data?.nodes?.filter((node) => node.roles.search && node.has_token) ?? [];
  const enableSearch = searchNodes.length > 0;

  if (enableSearch) {
    return <CommunityNodePostSearchResults query={query} />;
  }

  return <LocalPostSearchResults query={query} />;
}

function LocalPostSearchResults({ query }: PostSearchResultsProps) {
  const { t } = useTranslation();
  const { data: allPosts, isLoading } = usePosts();

  const searchResults = useQuery({
    queryKey: ['search', 'posts', query],
    queryFn: async () => {
      if (!query || !allPosts) return [];

      const searchTerm = query.toLowerCase();

      return allPosts.filter((post) => {
        const contentMatch = post.content.toLowerCase().includes(searchTerm);
        const authorNameMatch =
          post.author.name?.toLowerCase().includes(searchTerm) ||
          post.author.displayName?.toLowerCase().includes(searchTerm);

        return contentMatch || authorNameMatch;
      });
    },
    enabled: !!query && !!allPosts,
    staleTime: 0,
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
        <p className="text-lg font-medium">{t('search.noPostResults')}</p>
        <p className="text-muted-foreground mt-2">{t('search.noPostResultsDescription', { query })}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{t('search.postsFound', { count: results.length })}</p>
      <div className="space-y-4">
        {results.map((post) => (
          <SearchResultPost key={post.id} post={post} />
        ))}
      </div>
    </div>
  );
}

function CommunityNodePostSearchResults({ query }: PostSearchResultsProps) {
  const { t } = useTranslation();
  const { currentTopic, joinedTopics, topics } = useTopicStore();
  const trimmedQuery = query.trim();
  const debouncedQuery = useDebounce(trimmedQuery, 300);
  const fallbackTopicId = joinedTopics[0] ?? '';
  const topicId = currentTopic?.id ?? fallbackTopicId;
  const resolvedTopic = currentTopic ?? (topicId ? (topics.get(topicId) ?? null) : null);
  const topicName = resolvedTopic?.name;

  const searchQuery = useInfiniteQuery<
    CommunityNodeSearchResponse,
    Error,
    InfiniteData<CommunityNodeSearchResponse, string | null>,
    ['community-node', 'search', string, string],
    string | null
  >({
    queryKey: ['community-node', 'search', topicId, debouncedQuery],
    queryFn: ({ pageParam }) =>
      fetchCommunityNodeSearchPage({
        topicId,
        query: debouncedQuery,
        cursor: pageParam ?? null,
        limit: COMMUNITY_NODE_PAGE_SIZE,
      }),
    initialPageParam: null,
    getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
    enabled: Boolean(topicId) && debouncedQuery.length > 0,
    refetchOnWindowFocus: false,
    retry: false,
  });

  useEffect(() => {
    if (!searchQuery.isError) {
      return;
    }
    errorHandler.log('CommunityNode.search_failed', searchQuery.error, {
      context: 'PostSearchResults.communityNode',
      metadata: { topicId, query: debouncedQuery },
    });
  }, [searchQuery.error, searchQuery.isError, topicId, debouncedQuery]);

  const normalizedHits = useMemo(() => {
    const pages = searchQuery.data?.pages ?? [];
    return pages.flatMap((page) => page.items.map((item) => normalizeSearchHit(item)));
  }, [searchQuery.data]);

  const total = searchQuery.data?.pages?.[0]?.total ?? 0;
  const hasNextPage = Boolean(searchQuery.hasNextPage);
  const isInitialLoading = searchQuery.isLoading && normalizedHits.length === 0;

  if (!trimmedQuery) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        {t('search.enterKeyword')}
      </div>
    );
  }

  if (!topicId) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        {t('search.selectTopic')}
      </div>
    );
  }

  if (isInitialLoading) {
    return (
      <div className="flex justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (searchQuery.isError) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        {t('search.searchFailed')}
      </div>
    );
  }

  if (normalizedHits.length === 0) {
    return (
      <div className="text-center py-12" data-testid="community-node-search-empty">
        <p className="text-lg font-medium">{t('search.noPostResults')}</p>
        <p className="text-muted-foreground mt-2">{t('search.noPostResultsDescription', { query: trimmedQuery })}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4" data-testid="community-node-search-results">
      <div className="space-y-2">
        <div
          className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground"
          data-testid="community-node-search-summary"
        >
          <span>{t('search.postsFound', { count: total })}</span>
          {topicName && (
            <Badge variant="outline">
              {topicName} ({topicId})
            </Badge>
          )}
        </div>
        <p className="text-xs text-muted-foreground">{t('search.communityNodeSearch', { query: trimmedQuery })}</p>
      </div>
      <div className="space-y-4">
        {normalizedHits.map((hit, index) => (
          <CommunityNodeSearchResultCard
            key={hit.eventId || `${hit.topicId}-${index}`}
            hit={hit}
            index={index}
          />
        ))}
      </div>
      {hasNextPage && (
        <div className="flex justify-center pt-2">
          <Button
            variant="outline"
            onClick={() => searchQuery.fetchNextPage()}
            disabled={searchQuery.isFetchingNextPage}
            data-testid="community-node-search-load-more"
          >
            {searchQuery.isFetchingNextPage ? t('search.loading') : t('search.loadMore')}
          </Button>
        </div>
      )}
    </div>
  );
}

function CommunityNodeSearchResultCard({
  hit,
  index,
}: {
  hit: NormalizedSearchHit;
  index: number;
}) {
  const { t } = useTranslation();
  const title = hit.title || hit.summary || hit.content || t('search.searchResult', { index: index + 1 });
  const summary = hit.summary || hit.content;
  const createdAtText = hit.createdAt ? new Date(hit.createdAt * 1000).toLocaleString() : 'unknown';
  const authorLabel = hit.author ? `pubkey:${hit.author}` : t('search.unknownAuthor');

  return (
    <Card
      data-testid="community-node-search-result"
      data-event-id={hit.eventId}
      data-topic-id={hit.topicId}
    >
      <CardContent className="space-y-2 p-4">
        <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>{createdAtText}</span>
          {hit.topicId && <Badge variant="outline">{hit.topicId}</Badge>}
        </div>
        <div className="space-y-1">
          <p className="text-sm font-semibold">{title}</p>
          {summary && <p className="text-sm text-muted-foreground">{summary}</p>}
        </div>
        <p className="text-xs text-muted-foreground break-all">{authorLabel}</p>
      </CardContent>
    </Card>
  );
}

function SearchResultPost({ post }: { post: Post }) {
  return <PostCard post={post} />;
}
