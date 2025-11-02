import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { TauriApi } from '@/lib/api/tauri';
import { useAuthStore } from '@/stores';
import { Loader2, X } from 'lucide-react';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';

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
  const [content, setContent] = useState('');
  const queryClient = useQueryClient();
  const { currentUser } = useAuthStore();

  // 返信投稿の作成
  const replyMutation = useMutation({
    mutationFn: async (content: string) => {
      if (!content.trim()) {
        throw new Error('返信内容を入力してください');
      }

      // 返信を作成（Nostr NIP-10準拠）
      const tags = [
        ['e', postId, '', 'reply'], // 返信先の投稿ID
      ];

      if (topicId) {
        tags.push(['t', topicId]); // トピックタグ
      }

      await TauriApi.createPost({
        content: content.trim(),
        topic_id: topicId,
        tags,
      });
    },
    onSuccess: () => {
      setContent('');
      toast.success('返信を投稿しました');

      // データを再取得
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      if (topicId) {
        queryClient.invalidateQueries({ queryKey: ['posts', topicId] });
      }

      onSuccess?.();
    },
    onError: (error) => {
      errorHandler.log('Failed to post reply', error, {
        context: 'ReplyForm',
        showToast: true,
        toastTitle: '返信の投稿に失敗しました',
      });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (content.trim() && !replyMutation.isPending) {
      replyMutation.mutate(content);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleSubmit(e as React.FormEvent);
    }
  };

  // アバターのイニシャルを生成
  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  if (!currentUser) {
    return null;
  }

  const avatarSrc = resolveUserAvatarSrc(currentUser);

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <div className="flex gap-3">
        <Avatar className="h-8 w-8">
          <AvatarImage src={avatarSrc} />
          <AvatarFallback>
            {getInitials(currentUser.displayName || currentUser.name || 'U')}
          </AvatarFallback>
        </Avatar>
        <div className="flex-1 space-y-2">
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="返信を入力..."
            className="min-h-[80px] resize-none"
            autoFocus={autoFocus}
            disabled={replyMutation.isPending}
          />
          <div className="flex items-center justify-between">
            <p className="text-xs text-muted-foreground">Ctrl+Enter または ⌘+Enter で送信</p>
            <div className="flex gap-2">
              {onCancel && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={onCancel}
                  disabled={replyMutation.isPending}
                >
                  <X className="mr-1 h-4 w-4" />
                  キャンセル
                </Button>
              )}
              <Button type="submit" size="sm" disabled={!content.trim() || replyMutation.isPending}>
                {replyMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    投稿中...
                  </>
                ) : (
                  '返信する'
                )}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </form>
  );
}
