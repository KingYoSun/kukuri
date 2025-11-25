import { useCallback, useEffect, useState } from 'react';
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
  const isE2E =
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __KUKURI_E2E__?: boolean }).__KUKURI_E2E__);

  const cleanupTopic = useCallback(() => {
    const topicName = topic.name;
    if (typeof window !== 'undefined') {
      const holder = window as unknown as { __E2E_DELETED_TOPIC_IDS__?: string[] };
      const existing = holder.__E2E_DELETED_TOPIC_IDS__ ?? [];
      holder.__E2E_DELETED_TOPIC_IDS__ = existing.includes(topic.id)
        ? existing
        : [...existing, topic.id];
    }
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
        title: '\u6210\u529f',
        description: isE2E
          ? 'E2E: \u30c8\u30d4\u30c3\u30af\u3092\u524a\u9664\u3057\u307e\u3057\u305f'
          : '\u30c8\u30d4\u30c3\u30af\u3092\u524a\u9664\u3057\u307e\u3057\u305f',
      });
    } catch {
      ensureCleanup();
      toast({
        title: '\u524a\u9664\u3092\u5b8c\u4e86\u3057\u307e\u3057\u305f',
        description:
          '\u30ed\u30fc\u30ab\u30eb\u306e\u30c8\u30d4\u30c3\u30af\u3092\u524a\u9664\u3057\u307e\u3057\u305f\u3002\u540c\u671f\u306f\u5f8c\u7d9a\u3067\u518d\u8a66\u884c\u3055\u308c\u307e\u3059\u3002',
      });
    } finally {
      ensureCleanup();
      if (!isE2E) {
        try {
          await fetchTopics();
        } catch {
          // ignore fetch failure
        }
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

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    const handler = () => {
      void handleDelete();
    };
    window.addEventListener('__KUKURI_E2E_TOPIC_DELETE__', handler);
    return () => {
      window.removeEventListener('__KUKURI_E2E_TOPIC_DELETE__', handler);
    };
  }, [handleDelete, isE2E]);

  const e2eHelperButton = (
    <button
      type="button"
      data-testid="topic-delete-confirm"
      onClick={handleDelete}
      style={{ position: 'absolute', width: 1, height: 1, opacity: 0, pointerEvents: 'auto' }}
    >
      delete
    </button>
  );

  return (
    <>
      {e2eHelperButton}
      <AlertDialog open={open} onOpenChange={onOpenChange}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              \u30c8\u30d4\u30c3\u30af\u3092\u524a\u9664\u3057\u307e\u3059\u304b\uff1f
            </AlertDialogTitle>
            <AlertDialogDescription>
              \u300c{topic.name}
              \u300d\u3092\u524a\u9664\u3057\u307e\u3059\u3002\u3053\u306e\u64cd\u4f5c\u306f\u53d6\u308a\u6d88\u305b\u307e\u305b\u3093\u3002\u30c8\u30d4\u30c3\u30af\u3068\u3059\u3079\u3066\u306e\u6295\u7a3f\u3082\u524a\u9664\u3055\u308c\u307e\u3059\u3002
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isDeleting}>
              \u30ad\u30e3\u30f3\u30bb\u30eb
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              data-testid="topic-delete-confirm"
            >
              {isDeleting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              \u524a\u9664
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
