import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { Quote } from 'lucide-react';

import { PostActionComposer } from '@/components/posts/PostActionComposer';
import { usePostActionForm } from '@/components/posts/hooks/usePostActionForm';
import { Card, CardContent } from '@/components/ui/card';
import { TauriApi } from '@/lib/api/tauri';
import { resolveUserAvatarSrc, getUserInitials } from '@/lib/profile/avatarDisplay';
import { useAuthStore } from '@/stores';
import type { Post } from '@/stores';

interface QuoteFormProps {
  post: Post;
  onCancel?: () => void;
  onSuccess?: () => void;
  autoFocus?: boolean;
}

export function QuoteForm({ post, onCancel, onSuccess, autoFocus = true }: QuoteFormProps) {
  const { currentUser } = useAuthStore();
  const { content, setContent, isPending, handleSubmit, handleKeyboardSubmit } = usePostActionForm({
    submit: async (message: string) => {
      const quoteContent = `${message}\n\nnostr:${post.id}`;
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
        tags,
      });
    },
    successMessage: '引用投稿を作成しました',
    emptyErrorMessage: 'コメントを入力してください',
    errorContext: 'QuoteForm',
    errorToastTitle: '引用投稿の作成に失敗しました',
    invalidations: [
      { queryKey: ['timeline'] },
      post.topicId ? { queryKey: ['posts', post.topicId] } : null,
    ],
    onSuccess,
  });

  const timeAgo = formatDistanceToNow(new Date(post.created_at * 1000), {
    addSuffix: true,
    locale: ja,
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
      placeholder="コメントを追加..."
      autoFocus={autoFocus}
      isPending={isPending}
      submitLabel="引用して投稿"
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
                  {post.author.displayName || post.author.name || 'ユーザー'}
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
