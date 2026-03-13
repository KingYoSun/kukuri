import { useTranslation } from 'react-i18next';
import { useAuthStore, usePostStore } from '@/stores';
import { resolveUserAvatarSrc, getUserInitials } from '@/lib/profile/avatarDisplay';
import { usePostActionForm } from '@/components/posts/hooks/usePostActionForm';
import { PostActionComposer } from '@/components/posts/PostActionComposer';
import { TauriApi } from '@/lib/api/tauri';
import type { PostScope } from '@/stores/types';
import { v4 as uuidv4 } from 'uuid';

interface ReplyFormProps {
  postId: string;
  topicId?: string;
  threadUuid?: string | null;
  scope?: PostScope;
  onCancel?: () => void;
  onSuccess?: () => void;
  autoFocus?: boolean;
}

export function ReplyForm({
  postId,
  topicId,
  threadUuid,
  scope,
  onCancel,
  onSuccess,
  autoFocus = true,
}: ReplyFormProps) {
  const { t } = useTranslation();
  const { currentUser } = useAuthStore();
  const { createPost } = usePostStore();
  const { content, setContent, isPending, handleSubmit, handleKeyboardSubmit } = usePostActionForm({
    submit: async (message: string) => {
      if (topicId) {
        const createPostOptions: Parameters<typeof createPost>[2] = {
          replyTo: postId,
          scope,
        };
        if (threadUuid) {
          createPostOptions.threadUuid = threadUuid;
        }
        await createPost(message, topicId, {
          ...createPostOptions,
        });
        return;
      }
      const tags: string[][] = [['e', postId, '', 'reply']];
      await TauriApi.createPost({
        content: message,
        topic_id: topicId,
        thread_uuid: uuidv4(),
        tags,
        scope,
      });
    },
    successMessage: t('posts.reply.success'),
    emptyErrorMessage: t('posts.reply.contentRequired'),
    errorContext: 'ReplyForm',
    errorToastTitle: t('posts.reply.failed'),
    invalidations: [{ queryKey: ['timeline'] }, topicId ? { queryKey: ['posts', topicId] } : null],
    onSuccess,
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
      placeholder={t('posts.reply.placeholder')}
      autoFocus={autoFocus}
      isPending={isPending}
      submitLabel={t('posts.reply.submit')}
      onSubmit={handleSubmit}
      onContentChange={setContent}
      onShortcut={handleKeyboardSubmit}
      onCancel={onCancel}
      dataTestId="reply-composer-input"
      submitDataTestId="reply-submit-button"
    />
  );
}
