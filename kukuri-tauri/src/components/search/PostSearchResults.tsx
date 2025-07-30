import { useQuery } from '@tanstack/react-query';
import { PostCard } from '@/components/posts/PostCard';
import { Loader2 } from 'lucide-react';
import { usePosts } from '@/hooks/usePosts';
import type { Post } from '@/stores';

interface PostSearchResultsProps {
  query: string;
}

export function PostSearchResults({ query }: PostSearchResultsProps) {
  const { data: allPosts, isLoading } = usePosts();

  // クライアントサイドで投稿を検索
  const searchResults = useQuery({
    queryKey: ['search', 'posts', query],
    queryFn: async () => {
      if (!query || !allPosts) return [];

      const searchTerm = query.toLowerCase();

      // 投稿内容、作者名で検索
      return allPosts.filter((post) => {
        const contentMatch = post.content.toLowerCase().includes(searchTerm);
        const authorNameMatch =
          post.author.name?.toLowerCase().includes(searchTerm) ||
          post.author.displayName?.toLowerCase().includes(searchTerm);

        return contentMatch || authorNameMatch;
      });
    },
    enabled: !!query && !!allPosts,
    staleTime: 0, // 常に最新のデータで検索
  });

  if (!query) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        検索キーワードを入力してください
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
        <p className="text-lg font-medium">検索結果が見つかりませんでした</p>
        <p className="text-muted-foreground mt-2">「{query}」に一致する投稿はありません</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{results.length}件の投稿が見つかりました</p>
      <div className="space-y-4">
        {results.map((post) => (
          <SearchResultPost key={post.id} post={post} query={query} />
        ))}
      </div>
    </div>
  );
}

// 検索結果用の投稿カード
function SearchResultPost({ post }: { post: Post; query: string }) {
  return <PostCard post={post} />;
}
