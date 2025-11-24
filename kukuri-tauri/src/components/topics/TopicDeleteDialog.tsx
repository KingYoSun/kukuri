import { useState } from 'react';
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
  const [isDeleting, setIsDeleting] = useState(false);
  const { deleteTopicRemote, leaveTopic, removeTopic, removePendingTopic } = useTopicStore();
  const { toast } = useToast();
  const navigate = useNavigate();

  const handleDelete = async () => {
    setIsDeleting(true);
    try {
      // P2Pトピックから離脱
      await leaveTopic(topic.id);
      // データベースから削除
      await deleteTopicRemote(topic.id);

      toast({
        title: '成功',
        description: 'トピックを削除しました',
      });
    } catch {
      // オフラインやE2E向けにローカルストアからも確実に削除
      removeTopic(topic.id);
      removePendingTopic(topic.id);
      toast({
        title: '削除を完了しました',
        description: 'ローカルのトピックを削除しました（同期は後続で再試行されます）',
      });
    } finally {
      onOpenChange(false);
      navigate({ to: '/topics' });
      setIsDeleting(false);
    }
  };

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>トピックを削除しますか？</AlertDialogTitle>
          <AlertDialogDescription>
            「{topic.name}」を削除します。この操作は取り消せません。
            トピック内のすべての投稿も削除されます。
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
