import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Card, CardContent } from '@/components/ui/card';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { TauriApi } from '@/lib/api/tauri';
import { useAuthStore } from '@/stores';
import { Loader2, X, Quote } from 'lucide-react';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import type { Post } from '@/stores';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

interface QuoteFormProps {
  post: Post;
  onCancel?: () => void;
  onSuccess?: () => void;
  autoFocus?: boolean;
}

export function QuoteForm({ post, onCancel, onSuccess, autoFocus = true }: QuoteFormProps) {
  const [content, setContent] = useState('');
  const queryClient = useQueryClient();
  const { currentUser } = useAuthStore();

  // 引用投稿の作成
  const quoteMutation = useMutation({
    mutationFn: async (content: string) => {
      if (!content.trim()) {
        throw new Error('コメントを入力してください');
      }

      // 引用投稿を作成（Nostr NIP-10準拠）
      // 引用された投稿の内容を含める
      const quoteContent = `${content.trim()}\n\nnostr:${post.id}`;

      const tags = [
        ['e', post.id, '', 'mention'], // 引用元の投稿ID
        ['q', post.id], // 引用タグ（NIP-10）
      ];

      if (post.topicId) {
        tags.push(['t', post.topicId]); // トピックタグ
      }

      await TauriApi.createPost({
        content: quoteContent,
        topic_id: post.topicId,
        tags,
      });
    },
    onSuccess: () => {
      setContent('');
      toast.success('引用投稿を作成しました');

      // データを再取得
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      if (post.topicId) {
        queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      }

      onSuccess?.();
    },
    onError: (error) => {
      errorHandler.log('Failed to create quote post', error, {
        context: 'QuoteForm',
        showToast: true,
        toastTitle: '引用投稿の作成に失敗しました',
      });
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (content.trim() && !quoteMutation.isPending) {
      quoteMutation.mutate(content);
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

  // 時間表示のフォーマット
  const timeAgo = formatDistanceToNow(new Date(post.created_at * 1000), {
    addSuffix: true,
    locale: ja,
  });

  if (!currentUser) {
    return null;
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-3">
      <div className="flex gap-3">
        <Avatar className="h-8 w-8">
          <AvatarImage src={currentUser.picture} />
          <AvatarFallback>
            {getInitials(currentUser.displayName || currentUser.name || 'U')}
          </AvatarFallback>
        </Avatar>
        <div className="flex-1 space-y-3">
          <Textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="コメントを追加..."
            className="min-h-[80px] resize-none"
            autoFocus={autoFocus}
            disabled={quoteMutation.isPending}
          />

          {/* 引用元の投稿 */}
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

          <div className="flex items-center justify-between">
            <p className="text-xs text-muted-foreground">Ctrl+Enter または ⌘+Enter で送信</p>
            <div className="flex gap-2">
              {onCancel && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={onCancel}
                  disabled={quoteMutation.isPending}
                >
                  <X className="mr-1 h-4 w-4" />
                  キャンセル
                </Button>
              )}
              <Button type="submit" size="sm" disabled={!content.trim() || quoteMutation.isPending}>
                {quoteMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    投稿中...
                  </>
                ) : (
                  '引用して投稿'
                )}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </form>
  );
}
