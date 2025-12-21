import { useAuthStore, usePostStore } from '@/stores';
import { resolveUserAvatarSrc, getUserInitials } from '@/lib/profile/avatarDisplay';
import { usePostActionForm } from '@/components/posts/hooks/usePostActionForm';
import { PostActionComposer } from '@/components/posts/PostActionComposer';
import { TauriApi } from '@/lib/api/tauri';

interface ReplyFormProps {
  postId: string;
  topicId?: string;
  onCancel?: () => void;
  onSuccess?: () => void;
  autoFocus?: boolean;
}

export function ReplyForm({
  postId,
  topicId,
  onCancel,
  onSuccess,
  autoFocus = true,
}: ReplyFormProps) {
  const { currentUser } = useAuthStore();
  const { createPost } = usePostStore();
  const { content, setContent, isPending, handleSubmit, handleKeyboardSubmit } = usePostActionForm({
    submit: async (message: string) => {
      if (topicId) {
        await createPost(message, topicId, { replyTo: postId });
        return;
      }
      const tags: string[][] = [['e', postId, '', 'reply']];
      await TauriApi.createPost({
        content: message,
        topic_id: topicId,
        tags,
      });
    },
    successMessage: '返信を投稿しました',
    emptyErrorMessage: '返信内容を入力してください',
    errorContext: 'ReplyForm',
    errorToastTitle: '返信の投稿に失敗しました',
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
      placeholder="返信を入力..."
      autoFocus={autoFocus}
      isPending={isPending}
      submitLabel="返信する"
      onSubmit={handleSubmit}
      onContentChange={setContent}
      onShortcut={handleKeyboardSubmit}
      onCancel={onCancel}
      dataTestId="reply-composer-input"
      submitDataTestId="reply-submit-button"
    />
  );
}
