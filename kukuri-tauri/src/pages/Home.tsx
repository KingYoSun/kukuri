import { useState } from 'react';
import { useTimelinePosts } from '@/hooks/usePosts';
import { PostCard } from '@/components/posts/PostCard';
import { PostComposer } from '@/components/posts/PostComposer';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Loader2, PlusCircle } from 'lucide-react';
import { useTopicStore } from '@/stores/topicStore';

function Home() {
  const { data: posts, isLoading, error, refetch } = useTimelinePosts();
  const { joinedTopics } = useTopicStore();
  const [showComposer, setShowComposer] = useState(false);

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
          <AlertDescription>投稿の取得に失敗しました。リロードしてください。</AlertDescription>
        </Alert>
      </div>
    );
  }

  const handlePostSuccess = () => {
    setShowComposer(false);
    refetch(); // 投稿一覧を再取得
  };

  return (
    <div className="max-w-2xl mx-auto">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold">タイムライン</h2>
        {joinedTopics.length > 0 && !showComposer && (
          <Button onClick={() => setShowComposer(true)} size="sm">
            <PlusCircle className="h-4 w-4 mr-2" />
            投稿する
          </Button>
        )}
      </div>

      {showComposer && (
        <div className="mb-6">
          <PostComposer
            onSuccess={handlePostSuccess}
            onCancel={() => setShowComposer(false)}
          />
        </div>
      )}

      <div className="space-y-4">
        {posts && posts.length > 0 ? (
          posts.map((post) => <PostCard key={post.id} post={post} />)
        ) : (
          <Alert>
            <AlertDescription>
              {joinedTopics.length === 0
                ? 'トピックに参加すると、投稿が表示されます。'
                : 'まだ投稿がありません。最初の投稿をしてみましょう！'}
            </AlertDescription>
          </Alert>
        )}
      </div>
    </div>
  );
}

export default Home;
