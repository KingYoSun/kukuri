import { createFileRoute } from '@tanstack/react-router';
import { useState, useMemo } from 'react';
import { useTopicStore } from '@/stores';
import { usePostsByTopic } from '@/hooks';
import { PostCard } from '@/components/posts/PostCard';
import { PostComposer } from '@/components/posts/PostComposer';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Hash, PlusCircle, Loader2, MoreVertical, Edit, Trash2 } from 'lucide-react';
import { TopicMeshVisualization } from '@/components/TopicMeshVisualization';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { TopicFormModal } from '@/components/topics/TopicFormModal';
import { TopicDeleteDialog } from '@/components/topics/TopicDeleteDialog';

export const Route = createFileRoute('/topics/$topicId')({
  component: TopicPage,
});

function TopicPage() {
  const { topicId } = Route.useParams();
  const { topics, joinedTopics } = useTopicStore();
  const { data: posts, isLoading, refetch } = usePostsByTopic(topicId);
  const [showComposer, setShowComposer] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  const topic = topics.get(topicId);
  const isJoined = useMemo(() => joinedTopics.includes(topicId), [joinedTopics, topicId]);

  if (!topic) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">トピックが見つかりません</p>
      </div>
    );
  }

  const handlePostSuccess = () => {
    setShowComposer(false);
    refetch();
  };

  return (
    <div className="space-y-6">
      <div className="bg-card rounded-lg p-6 border">
        <div className="flex items-center gap-3 mb-4">
          <Hash className="h-8 w-8 text-primary" />
          <h1 className="text-3xl font-bold">{topic.name}</h1>
        </div>
        {topic.description && <p className="text-muted-foreground mb-4">{topic.description}</p>}
        <div className="flex items-center justify-between mt-4">
          <div className="flex items-center gap-4 text-sm text-muted-foreground">
            <span>{topic.memberCount} メンバー</span>
            <span>•</span>
            <span>
              最終更新:{' '}
              {topic.lastActive
                ? new Date(topic.lastActive * 1000).toLocaleDateString('ja-JP')
                : '-'}
            </span>
          </div>
          <div className="flex items-center gap-2">
            {isJoined && !showComposer && (
              <Button onClick={() => setShowComposer(true)} size="sm">
                <PlusCircle className="h-4 w-4 mr-2" />
                投稿する
              </Button>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => setShowEditModal(true)}>
                  <Edit className="h-4 w-4 mr-2" />
                  編集
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => setShowDeleteDialog(true)}
                  className="text-destructive"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  削除
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>

      <TopicMeshVisualization topicId={topicId} />

      {showComposer && (
        <PostComposer
          topicId={topicId}
          onSuccess={handlePostSuccess}
          onCancel={() => setShowComposer(false)}
        />
      )}

      <div className="space-y-4">
        {isLoading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-8 w-8 animate-spin" />
          </div>
        ) : !posts || posts.length === 0 ? (
          <Alert>
            <AlertDescription>
              {isJoined
                ? 'まだ投稿がありません。最初の投稿をしてみましょう！'
                : 'このトピックに参加すると投稿が表示されます。'}
            </AlertDescription>
          </Alert>
        ) : (
          posts.map((post) => <PostCard key={post.id} post={post} />)
        )}
      </div>

      <TopicFormModal
        open={showEditModal}
        onOpenChange={setShowEditModal}
        topic={topic}
        mode="edit"
      />

      <TopicDeleteDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog} topic={topic} />
    </div>
  );
}
