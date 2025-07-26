import { createFileRoute } from '@tanstack/react-router';
import { useTopicStore } from '@/stores';
import { usePostsByTopic } from '@/hooks';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Hash, MessageCircle, Heart } from 'lucide-react';

export const Route = createFileRoute('/topics/$topicId')({
  component: TopicPage,
});

function TopicPage() {
  const { topicId } = Route.useParams();
  const { topics } = useTopicStore();
  const { data: posts, isLoading } = usePostsByTopic(topicId);

  const topic = topics.get(topicId);

  if (!topic) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">トピックが見つかりません</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="bg-card rounded-lg p-6 border">
        <div className="flex items-center gap-3 mb-4">
          <Hash className="h-8 w-8 text-primary" />
          <h1 className="text-3xl font-bold">{topic.name}</h1>
        </div>
        {topic.description && <p className="text-muted-foreground mb-4">{topic.description}</p>}
        <div className="flex items-center gap-4 text-sm text-muted-foreground">
          <span>{topic.memberCount} メンバー</span>
          <span>•</span>
          <span>
            最終更新:{' '}
            {topic.lastActive ? new Date(topic.lastActive).toLocaleDateString('ja-JP') : '-'}
          </span>
        </div>
      </div>

      <div className="space-y-4">
        {isLoading ? (
          <Card className="p-6 text-center">
            <p className="text-muted-foreground">読み込み中...</p>
          </Card>
        ) : !posts || posts.length === 0 ? (
          <Card className="p-6 text-center">
            <p className="text-muted-foreground">まだ投稿がありません</p>
          </Card>
        ) : (
          posts.map((post) => (
            <Card key={post.id} className="p-6">
              <div className="flex items-start gap-4">
                <div className="flex-1">
                  <p className="text-sm text-muted-foreground mb-2">
                    {post.author.name || post.author.npub.slice(0, 8) + '...'} •{' '}
                    {new Date(post.created_at * 1000).toLocaleString('ja-JP')}
                  </p>
                  <p className="whitespace-pre-wrap">{post.content}</p>
                  <div className="flex items-center gap-4 mt-4">
                    <Button variant="ghost" size="sm">
                      <MessageCircle className="h-4 w-4 mr-2" />
                      返信
                    </Button>
                    <Button variant="ghost" size="sm">
                      <Heart className="h-4 w-4 mr-2" />
                      いいね
                    </Button>
                  </div>
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
    </div>
  );
}
