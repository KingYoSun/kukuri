import { useTimelinePosts, usePostsByTopic } from '@/hooks/usePosts';
import { PostCard } from '@/components/posts/PostCard';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Loader2, PlusCircle } from 'lucide-react';
import { useTopicStore } from '@/stores/topicStore';
import { useComposerStore } from '@/stores/composerStore';

function Home() {
  const { joinedTopics, currentTopic } = useTopicStore();

  // currentTopicがある場合はそのトピックの投稿を、ない場合は全体のタイムラインを取得
  const timelineQuery = useTimelinePosts();
  const topicQuery = usePostsByTopic(currentTopic?.id || '');

  const { data: posts, isLoading, error, refetch } = currentTopic ? topicQuery : timelineQuery;
  const { openComposer, isOpen } = useComposerStore();

  if (isLoading) {
    return (
      <div className="max-w-2xl mx-auto">
        <h2 className="text-2xl font-bold mb-6">
          {currentTopic ? currentTopic.name : 'タイムライン'}
        </h2>
        <div className="flex justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin" data-testid="loader" />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-2xl mx-auto">
        <h2 className="text-2xl font-bold mb-6">
          {currentTopic ? currentTopic.name : 'タイムライン'}
        </h2>
        <Alert variant="destructive">
          <AlertDescription>投稿の取得に失敗しました。リロードしてください。</AlertDescription>
        </Alert>
      </div>
    );
  }

  const handleOpenComposer = () => {
    openComposer({
      topicId: currentTopic?.id,
      onSuccess: () => {
        refetch();
      },
    });
  };

  return (
    <div className="max-w-2xl mx-auto" data-testid="home-page">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold">{currentTopic ? currentTopic.name : 'タイムライン'}</h2>
        {joinedTopics.length > 0 && !isOpen && (
          <Button onClick={handleOpenComposer} size="sm" data-testid="create-post-button">
            <PlusCircle className="h-4 w-4 mr-2" />
            投稿する
          </Button>
        )}
      </div>

      <div className="space-y-4" data-testid="posts-list">
        {posts && posts.length > 0 ? (
          posts.map((post) => (
            <PostCard key={post.id} post={post} data-testid={`post-${post.id}`} />
          ))
        ) : (
          <Alert>
            <AlertDescription>
              {joinedTopics.length === 0
                ? 'トピックに参加すると、投稿が表示されます。'
                : currentTopic
                  ? `${currentTopic.name}にはまだ投稿がありません。最初の投稿をしてみましょう！`
                  : 'まだ投稿がありません。最初の投稿をしてみましょう！'}
            </AlertDescription>
          </Alert>
        )}
      </div>
    </div>
  );
}

export default Home;
