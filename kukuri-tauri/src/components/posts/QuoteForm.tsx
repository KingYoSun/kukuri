import { useTranslation } from 'react-i18next';
import { formatDistanceToNow } from 'date-fns';
import { Quote } from 'lucide-react';

import { PostActionComposer } from '@/components/posts/PostActionComposer';
import { usePostActionForm } from '@/components/posts/hooks/usePostActionForm';
import { Card, CardContent } from '@/components/ui/card';
import { TauriApi } from '@/lib/api/tauri';
import { resolveUserAvatarSrc, getUserInitials } from '@/lib/profile/avatarDisplay';
import { useAuthStore, usePostStore } from '@/stores';
import type { Post } from '@/stores';
import { getDateFnsLocale } from '@/i18n';
import { v4 as uuidv4 } from 'uuid';

interface QuoteFormProps {
  post: Post;
  onCancel?: () => void;
  onSuccess?: () => void;
  autoFocus?: boolean;
}

export function QuoteForm({ post, onCancel, onSuccess, autoFocus = true }: QuoteFormProps) {
  const { t } = useTranslation();
  const { currentUser } = useAuthStore();
  const { createPost } = usePostStore();
  const { content, setContent, isPending, handleSubmit, handleKeyboardSubmit } = usePostActionForm({
    submit: async (message: string) => {
      const quoteContent = `${message}\n\nnostr:${post.id}`;
      if (post.topicId) {
        await createPost(quoteContent, post.topicId, {
          quotedPost: post.id,
          scope: post.scope,
        });
        return;
      }
      const tags: string[][] = [
        ['e', post.id, '', 'mention'],
        ['q', post.id],
      ];
      if (post.topicId) {
        tags.push(['t', post.topicId]);
      }
      await TauriApi.createPost({
        content: quoteContent,
        topic_id: post.topicId,
        thread_uuid: post.threadUuid ?? uuidv4(),
        tags,
        scope: post.scope,
      });
    },
    successMessage: t('posts.quote.success'),
    emptyErrorMessage: t('posts.quote.contentRequired'),
    errorContext: 'QuoteForm',
    errorToastTitle: t('posts.quote.failed'),
    invalidations: [
      { queryKey: ['timeline'] },
      post.topicId ? { queryKey: ['posts', post.topicId] } : null,
    ],
    onSuccess,
  });

  const timeAgo = formatDistanceToNow(new Date(post.created_at * 1000), {
    addSuffix: true,
    locale: getDateFnsLocale(),
  });

  if (!currentUser) {
    return null;
  }

  const avatarSrc = resolveUserAvatarSrc(currentUser);
  const initials = getUserInitials(currentUser.displayName || currentUser.name);

  return (
    <PostActionComposer
      avatarSrc={avatarSrc}
      initials={initials}
      content={content}
      placeholder={t('posts.quote.placeholder')}
      autoFocus={autoFocus}
      isPending={isPending}
      submitLabel={t('posts.quote.submit')}
      onSubmit={handleSubmit}
      onContentChange={setContent}
      onShortcut={handleKeyboardSubmit}
      onCancel={onCancel}
      dataTestId="quote-composer-input"
      submitDataTestId="quote-submit-button"
    >
      <Card className="bg-muted/50 border-muted">
        <CardContent className="p-3">
          <div className="flex items-start gap-2 text-sm">
            <Quote className="h-4 w-4 text-muted-foreground mt-0.5" />
            <div className="flex-1 space-y-1">
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span className="font-medium">
                  {post.author.displayName || post.author.name || t('posts.user')}
                </span>
                <span>·</span>
                <span>{timeAgo}</span>
              </div>
              <p className="text-sm line-clamp-3 whitespace-pre-wrap">{post.content}</p>
            </div>
          </div>
        </CardContent>
      </Card>
    </PostActionComposer>
  );
}
