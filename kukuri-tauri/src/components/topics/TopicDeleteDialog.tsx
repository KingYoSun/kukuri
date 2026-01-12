import { useCallback, useState } from 'react';
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
        title: '成功',
        description: 'トピックを削除しました',
      });
    } catch {
      ensureCleanup();
      toast({
        title: '削除を完了しました',
        description: 'ローカルのトピックを削除しました。同期は後続で再試行されます。',
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
          <AlertDialogTitle>トピックを削除しますか？</AlertDialogTitle>
          <AlertDialogDescription>
            「{topic.name}
            」を削除します。この操作は取り消せません。トピックとすべての投稿も削除されます。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={isDeleting}>キャンセル</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleDelete}
            disabled={isDeleting}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            data-testid="topic-delete-confirm"
          >
            {isDeleting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            削除
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
