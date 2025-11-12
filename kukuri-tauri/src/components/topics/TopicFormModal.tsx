import { useState, useEffect } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
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
import { Button } from '@/components/ui/button';
import { useTopicStore } from '@/stores/topicStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { useComposerStore } from '@/stores/composerStore';
import { useToast } from '@/hooks/use-toast';
import { Loader2 } from 'lucide-react';
import type { Topic } from '@/stores';

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
  const watchPendingTopic = useComposerStore((state) => state.watchPendingTopic);
  const { toast } = useToast();

  const form = useForm<TopicFormValues>({
    resolver: zodResolver(topicFormSchema),
    defaultValues: {
      name: '',
      description: '',
    },
  });

  // トピック編集時の初期値設定
  useEffect(() => {
    if (mode === 'edit' && topic) {
      form.reset({
        name: topic.name,
        description: topic.description || '',
      });
    }
  }, [mode, topic, form]);

  const onSubmit = async (values: TopicFormValues) => {
    setIsSubmitting(true);
    try {
      if (!isOnline && (mode === 'create' || mode === 'create-from-composer')) {
        try {
          const pending = await queueTopicCreation(values.name, values.description || '');
          if (mode === 'create-from-composer' || autoJoin) {
            watchPendingTopic(pending.pending_id);
          }
          toast({
            title: '作成を予約しました',
            description: 'オフラインのため接続復帰後に自動で同期されます。',
          });
          onOpenChange(false);
          form.reset();
        } finally {
          setIsSubmitting(false);
        }
        return;
      }

      if (mode === 'create' || mode === 'create-from-composer') {
        const createdTopic = await createTopic(values.name, values.description || '');

        if (mode === 'create-from-composer' || autoJoin) {
          try {
            await joinTopic(createdTopic.id);
          } catch (error) {
            toast({
              title: '注意',
              description: 'トピックの参加に失敗しました。再試行してください。',
              variant: 'destructive',
            });
            throw error;
          }
        }

        toast({
          title: '成功',
          description: 'トピックを作成しました',
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
    } catch {
      // エラーハンドリングはストアで行われる
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
                ? '投稿用の新しいトピック'
                : '新しいトピックを作成'}
          </DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? 'トピックの名前と説明を入力してください'
              : mode === 'create-from-composer'
                ? '投稿を続けるためのトピックを作成します。作成後すぐに参加します。'
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
                    <Input placeholder="例: プログラミング" {...field} disabled={isSubmitting} />
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
                      placeholder="このトピックについての説明を入力してください"
                      rows={3}
                      {...field}
                      disabled={isSubmitting}
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
              <Button type="submit" disabled={isSubmitting}>
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
