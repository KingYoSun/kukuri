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

interface TopicFormModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  topic?: Topic;
  mode: 'create' | 'edit';
}

export function TopicFormModal({ open, onOpenChange, topic, mode }: TopicFormModalProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { createTopic, updateTopicRemote } = useTopicStore();
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
      if (mode === 'create') {
        await createTopic(values.name, values.description || '');
        toast({
          title: '成功',
          description: 'トピックを作成しました',
        });
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
          <DialogTitle>{mode === 'create' ? '新しいトピックを作成' : 'トピックを編集'}</DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? 'トピックの名前と説明を入力してください'
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
                {mode === 'create' ? '作成' : '更新'}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
