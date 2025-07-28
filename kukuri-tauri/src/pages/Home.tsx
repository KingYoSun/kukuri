import { useTimelinePosts } from '@/hooks/usePosts';
import { PostCard } from '@/components/posts/PostCard';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2 } from 'lucide-react';

function Home() {
  const { data: posts, isLoading, error } = useTimelinePosts();

  if (isLoading) {
    return (
      <div className="max-w-2xl mx-auto">
        <h2 className="text-2xl font-bold mb-6">タイムライン</h2>
        <div className="flex justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin" data-testid="loader" />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-2xl mx-auto">
        <h2 className="text-2xl font-bold mb-6">タイムライン</h2>
        <Alert variant="destructive">
          <AlertDescription>
            投稿の取得に失敗しました。リロードしてください。
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold mb-6">タイムライン</h2>

      <div className="space-y-4">
        {posts && posts.length > 0 ? (
          posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))
        ) : (
          <Alert>
            <AlertDescription>
              まだ投稿がありません。最初の投稿をしてみましょう！
            </AlertDescription>
          </Alert>
        )}
      </div>
    </div>
  );
}

export default Home;
