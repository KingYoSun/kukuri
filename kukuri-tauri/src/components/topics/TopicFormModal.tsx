import { useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Loader2 } from 'lucide-react';
import * as z from 'zod';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { useToast } from '@/hooks/use-toast';
import { errorHandler } from '@/lib/errorHandler';
import { useComposerStore } from '@/stores/composerStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { useTopicStore } from '@/stores/topicStore';
import type { Topic } from '@/stores';

const isNavigatorOnline = () => (typeof navigator !== 'undefined' ? navigator.onLine : true);

let offlineEventFlag = !isNavigatorOnline();

if (typeof window !== 'undefined') {
  window.addEventListener('offline', () => {
    offlineEventFlag = true;
    useOfflineStore.getState().setOnlineStatus(false);
  });
  window.addEventListener('online', () => {
    offlineEventFlag = false;
    useOfflineStore.getState().setOnlineStatus(true);
  });
}

const topicFormSchema = z.object({
  name: z
    .string()
    .min(1, 'トピック名は必須です')
    .max(50, 'トピック名は50文字以内で入力してください'),
  description: z.string().max(200, '説明は200文字以内で入力してください').optional(),
});

type TopicFormValues = z.infer<typeof topicFormSchema>;

type TopicFormMode = 'create' | 'edit' | 'create-from-composer';

interface TopicFormModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  topic?: Topic;
  mode?: TopicFormMode;
  onCreated?: (topic: Topic) => void;
  autoJoin?: boolean;
}

export function TopicFormModal({
  open,
  onOpenChange,
  topic,
  mode = 'create',
  onCreated,
  autoJoin = false,
}: TopicFormModalProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { createTopic, updateTopicRemote, joinTopic, queueTopicCreation } = useTopicStore();
  const isOnline = useOfflineStore((state) => state.isOnline);
  const [forceOffline, setForceOffline] = useState(() => offlineEventFlag);
  const watchPendingTopic = useComposerStore((state) => state.watchPendingTopic);
  const { toast } = useToast();

  const form = useForm<TopicFormValues>({
    resolver: zodResolver(topicFormSchema),
    defaultValues: {
      name: '',
      description: '',
    },
  });

  useEffect(() => {
    if (mode === 'edit' && topic) {
      form.reset({
        name: topic.name,
        description: topic.description || '',
      });
    }
  }, [mode, topic, form]);

  useEffect(() => {
    setForceOffline(offlineEventFlag || !isOnline || !isNavigatorOnline());
  }, [isOnline]);

  useEffect(() => {
    const handleOffline = () => {
      offlineEventFlag = true;
      setForceOffline(true);
      useOfflineStore.getState().setOnlineStatus(false);
    };
    const handleOnline = () => {
      offlineEventFlag = false;
      setForceOffline(false);
      useOfflineStore.getState().setOnlineStatus(true);
    };
    window.addEventListener('offline', handleOffline, { capture: true });
    window.addEventListener('online', handleOnline, { capture: true });
    document.addEventListener('offline', handleOffline, { capture: true });
    document.addEventListener('online', handleOnline, { capture: true });
    return () => {
      window.removeEventListener('offline', handleOffline, {
        capture: true,
      } as EventListenerOptions);
      window.removeEventListener('online', handleOnline, { capture: true } as EventListenerOptions);
      document.removeEventListener('offline', handleOffline, {
        capture: true,
      } as EventListenerOptions);
      document.removeEventListener('online', handleOnline, {
        capture: true,
      } as EventListenerOptions);
    };
  }, []);

  const onSubmit = async (values: TopicFormValues) => {
    setIsSubmitting(true);
    const offlineMode = forceOffline || offlineEventFlag || !isOnline || !isNavigatorOnline();

    try {
      if (offlineMode && (mode === 'create' || mode === 'create-from-composer')) {
        const pending = await queueTopicCreation(values.name, values.description || '');
        if (mode === 'create-from-composer' || autoJoin) {
          watchPendingTopic(pending.pending_id);
        }
        toast({
          title: '作成をキューに追加しました',
          description: 'オフラインのため接続回復後に自動で処理されます。',
        });
        onOpenChange(false);
        form.reset();
        return;
      }

      if (mode === 'create' || mode === 'create-from-composer') {
        const createdTopic = await createTopic(values.name, values.description || '');

        if (mode === 'create-from-composer' || autoJoin) {
          try {
            await joinTopic(createdTopic.id);
          } catch (error) {
            toast({
              title: 'エラー',
              description: 'トピックの参加に失敗しました。再試行してください。',
              variant: 'destructive',
            });
            throw error;
          }
        }

        toast({
          title: '成功',
          description: 'トピックを作成し参加しました',
        });
        onCreated?.(createdTopic);
      } else if (mode === 'edit' && topic) {
        await updateTopicRemote(topic.id, values.name, values.description || '');
        toast({
          title: '成功',
          description: 'トピックを更新しました',
        });
      }

      onOpenChange(false);
      form.reset();
    } catch (error) {
      try {
        const pending = await queueTopicCreation(values.name, values.description || '');
        if (mode === 'create-from-composer' || autoJoin) {
          watchPendingTopic(pending.pending_id);
        }
        toast({
          title: '作成をオフラインでキューしました',
          description: '接続復旧後に同期します。',
        });
        onOpenChange(false);
        form.reset();
      } catch (fallbackError) {
        errorHandler.log('Failed to create topic', fallbackError || error, {
          context: 'TopicFormModal.onSubmit',
          showToast: true,
          toastTitle: 'トピックの作成に失敗しました',
        });
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {mode === 'edit'
              ? 'トピックを編集'
              : mode === 'create-from-composer'
                ? '投稿用の新しいトピックを追加'
                : '新しいトピックを追加'}
          </DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? 'トピックの名前と説明を入力してください'
              : mode === 'create-from-composer'
                ? '投稿に紐づけるためのトピックを作成し、選択状態で追加します。'
                : 'トピックの情報を更新します'}
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>トピック名 *</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="例: プログラミング"
                      {...field}
                      disabled={isSubmitting}
                      data-testid="topic-name-input"
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="description"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>説明</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder="このトピックに関する説明を入力してください"
                      rows={3}
                      {...field}
                      disabled={isSubmitting}
                      data-testid="topic-description-input"
                    />
                  </FormControl>
                  <FormDescription>200文字以内で入力してください</FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={isSubmitting}
              >
                キャンセル
              </Button>
              <Button type="submit" disabled={isSubmitting} data-testid="topic-submit-button">
                {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {mode === 'create' || mode === 'create-from-composer' ? '作成' : '更新'}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
