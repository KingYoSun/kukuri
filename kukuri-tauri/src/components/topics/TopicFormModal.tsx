import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
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

const createTopicFormSchema = (t: (key: string) => string) => z.object({
  name: z
    .string()
    .min(1, t('topics.form.nameRequired'))
    .max(50, t('topics.form.nameMaxLength')),
  description: z.string().max(200, t('topics.form.descriptionMaxLength')).optional(),
});

type TopicFormValues = {
  name: string;
  description?: string;
};

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
  const { t } = useTranslation();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { createTopic, updateTopicRemote, joinTopic, queueTopicCreation } = useTopicStore();
  const isOnline = useOfflineStore((state) => state.isOnline);
  const [forceOffline, setForceOffline] = useState(() => offlineEventFlag);
  const watchPendingTopic = useComposerStore((state) => state.watchPendingTopic);
  const { toast } = useToast();

  const form = useForm<TopicFormValues>({
    resolver: zodResolver(createTopicFormSchema(t)),
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
          title: t('topics.form.queuedTitle'),
          description: t('topics.form.queuedDescription'),
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
              title: t('topics.form.errorTitle'),
              description: t('topics.form.joinFailedDescription'),
              variant: 'destructive',
            });
            throw error;
          }
        }

        toast({
          title: t('topics.form.successTitle'),
          description: t('topics.form.createSuccessDescription'),
        });
        onCreated?.(createdTopic);
      } else if (mode === 'edit' && topic) {
        await updateTopicRemote(topic.id, values.name, values.description || '');
        toast({
          title: t('topics.form.successTitle'),
          description: t('topics.form.updateSuccessDescription'),
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
          title: t('topics.form.offlineQueuedTitle'),
          description: t('topics.form.offlineQueuedDescription'),
        });
        onOpenChange(false);
        form.reset();
      } catch (fallbackError) {
        errorHandler.log('Failed to create topic', fallbackError || error, {
          context: 'TopicFormModal.onSubmit',
          showToast: true,
          toastTitle: t('topics.form.createFailed'),
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
              ? t('topics.form.editTitle')
              : mode === 'create-from-composer'
                ? t('topics.form.createFromComposerTitle')
                : t('topics.form.createTitle')}
          </DialogTitle>
          <DialogDescription>
            {mode === 'create'
              ? t('topics.form.createDescription')
              : mode === 'create-from-composer'
                ? t('topics.form.createFromComposerDescription')
                : t('topics.form.editDescription')}
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>{t('topics.form.nameLabel')}</FormLabel>
                  <FormControl>
                    <Input
                      placeholder={t('topics.form.namePlaceholder')}
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
                  <FormLabel>{t('topics.form.descriptionLabel')}</FormLabel>
                  <FormControl>
                    <Textarea
                      placeholder={t('topics.form.descriptionPlaceholder')}
                      rows={3}
                      {...field}
                      disabled={isSubmitting}
                      data-testid="topic-description-input"
                    />
                  </FormControl>
                  <FormDescription>{t('topics.form.descriptionHint')}</FormDescription>
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
                {t('topics.form.cancel')}
              </Button>
              <Button type="submit" disabled={isSubmitting} data-testid="topic-submit-button">
                {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {mode === 'create' || mode === 'create-from-composer' ? t('topics.form.create') : t('topics.form.update')}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
