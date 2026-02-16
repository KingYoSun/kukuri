import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQueryClient } from '@tanstack/react-query';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { useTopicStore } from '@/stores/topicStore';
import { useToast } from '@/hooks/use-toast';
import { useNavigate } from '@tanstack/react-router';
import { Loader2 } from 'lucide-react';
import type { Topic } from '@/stores';

interface TopicDeleteDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  topic: Topic;
}

export function TopicDeleteDialog({ open, onOpenChange, topic }: TopicDeleteDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [isDeleting, setIsDeleting] = useState(false);
  const { deleteTopicRemote, leaveTopic, removeTopic, removePendingTopic, fetchTopics } =
    useTopicStore();
  const { toast } = useToast();
  const navigate = useNavigate();

  const cleanupTopic = useCallback(() => {
    const topicName = topic.name;
    const state = useTopicStore.getState();
    state.pendingTopics.forEach((pending) => {
      if (pending.pending_id === topic.id || pending.name === topicName) {
        removePendingTopic(pending.pending_id);
      }
    });
    state.topics.forEach((item) => {
      if (item.id === topic.id || item.name === topicName) {
        removeTopic(item.id);
      }
    });
    queryClient.setQueryData<Topic[] | undefined>(['topics'], (previous) =>
      Array.isArray(previous)
        ? previous.filter((item) => item.id !== topic.id && item.name !== topicName)
        : previous,
    );
  }, [queryClient, removePendingTopic, removeTopic, topic.id, topic.name]);

  const handleDelete = useCallback(async () => {
    setIsDeleting(true);
    let cleanedUp = false;
    const ensureCleanup = () => {
      if (cleanedUp) {
        return;
      }
      cleanupTopic();
      cleanedUp = true;
    };
    try {
      await leaveTopic(topic.id);
      await deleteTopicRemote(topic.id);
      toast({
        title: t('topics.success'),
        description: t('topics.deleteSuccess'),
      });
    } catch {
      ensureCleanup();
      toast({
        title: t('topics.deleteCompleted'),
        description: t('topics.deleteCompletedDescription'),
      });
    } finally {
      ensureCleanup();
      try {
        await fetchTopics();
      } catch {
        // ignore fetch failure
      }
      queryClient.invalidateQueries({ queryKey: ['topics'] });
      queryClient.invalidateQueries({ queryKey: ['topic', topic.id] });
      onOpenChange(false);
      await navigate({ to: '/topics', replace: true });
      setIsDeleting(false);
    }
  }, [
    cleanupTopic,
    deleteTopicRemote,
    fetchTopics,
    leaveTopic,
    navigate,
    onOpenChange,
    toast,
    topic.id,
  ]);

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t('topics.deleteTopicTitle')}</AlertDialogTitle>
          <AlertDialogDescription>
            {t('topics.deleteTopicDescription', { name: topic.name })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={isDeleting}>{t('common.cancel')}</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleDelete}
            disabled={isDeleting}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            data-testid="topic-delete-confirm"
          >
            {isDeleting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {t('common.delete')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
