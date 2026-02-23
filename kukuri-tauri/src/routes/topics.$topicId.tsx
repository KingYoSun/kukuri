import { useTranslation } from 'react-i18next';
import { createFileRoute } from '@tanstack/react-router';
import { useState, useMemo } from 'react';
import { useTopicStore } from '@/stores';
import { useTopicTimeline } from '@/hooks';
import { TimelineThreadCard } from '@/components/posts/TimelineThreadCard';
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
import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';
import i18n from '@/i18n';

export const Route = createFileRoute('/topics/$topicId')({
  component: TopicPage,
});

function TopicPage() {
  const { t } = useTranslation();
  const { topicId } = Route.useParams();
  const { topics, joinedTopics, currentTopic, pendingTopics } = useTopicStore();
  const { data: timelineEntries, isLoading, refetch } = useTopicTimeline(topicId);
  const [showComposer, setShowComposer] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  const pendingTopic = pendingTopics.get(topicId);
  const isPublicTopic = topicId === DEFAULT_PUBLIC_TOPIC_ID;
  const topic = topics.get(topicId) ??
    (currentTopic?.id === topicId ? currentTopic : undefined) ??
    (pendingTopic
      ? {
          id: pendingTopic.pending_id,
          name: pendingTopic.name,
          description: isPublicTopic
            ? i18n.t('topics.publicTimeline')
            : (pendingTopic.description ?? ''),
          tags: [],
          memberCount: 0,
          postCount: 0,
          lastActive: pendingTopic.updated_at ?? pendingTopic.created_at,
          isActive: true,
          createdAt: new Date((pendingTopic.created_at ?? Math.floor(Date.now() / 1000)) * 1000),
          visibility: 'public',
          isJoined: true,
        }
      : undefined) ?? {
      id: topicId,
      name: topicId,
      description: isPublicTopic ? i18n.t('topics.publicTimeline') : '',
      tags: [],
      memberCount: 0,
      postCount: 0,
      lastActive: Math.floor(Date.now() / 1000),
      isActive: true,
      createdAt: new Date(),
      visibility: 'public',
      isJoined: joinedTopics.includes(topicId),
    };

  const isJoined = useMemo(() => joinedTopics.includes(topicId), [joinedTopics, topicId]);

  if (!topic) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-muted-foreground">{t('topics.notFound')}</p>
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
            <span>{t('topics.members', { count: topic.memberCount })}</span>
            <span>•</span>
            <span>
              {t('topics.lastUpdated')}:{' '}
              {topic.lastActive ? new Date(topic.lastActive * 1000).toLocaleDateString() : '-'}
            </span>
          </div>
          <div className="flex items-center gap-2">
            {isJoined && !showComposer && (
              <Button
                onClick={() => setShowComposer(true)}
                size="sm"
                data-testid="create-post-button"
              >
                <PlusCircle className="h-4 w-4 mr-2" />
                {t('topics.createPost')}
              </Button>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" data-testid="topic-actions-menu">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={() => setShowEditModal(true)}>
                  <Edit className="h-4 w-4 mr-2" />
                  {t('common.edit')}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onSelect={() => setShowDeleteDialog(true)}
                  className="text-destructive"
                  data-testid="topic-delete-menu"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  {t('common.delete')}
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
        ) : !timelineEntries || timelineEntries.length === 0 ? (
          <Alert>
            <AlertDescription>
              {isJoined ? t('topics.noPostsYet') : t('topics.joinToSeePosts')}
            </AlertDescription>
          </Alert>
        ) : (
          timelineEntries.map((entry) => (
            <TimelineThreadCard key={entry.threadUuid} entry={entry} />
          ))
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
