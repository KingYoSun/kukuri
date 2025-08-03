import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Heart, MessageCircle, Repeat2, Share, Bookmark, Quote } from 'lucide-react';
import type { Post } from '@/stores';
import { useBookmarkStore } from '@/stores';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';
import { ReplyForm } from './ReplyForm';
import { QuoteForm } from './QuoteForm';
import { ReactionPicker } from './ReactionPicker';
import { Collapsible, CollapsibleContent } from '@/components/ui/collapsible';

interface PostCardProps {
  post: Post;
}

export function PostCard({ post }: PostCardProps) {
  const [showReplyForm, setShowReplyForm] = useState(false);
  const [showQuoteForm, setShowQuoteForm] = useState(false);
  const queryClient = useQueryClient();
  const { isBookmarked, toggleBookmark, fetchBookmarks } = useBookmarkStore();
  const isPostBookmarked = isBookmarked(post.id);

  // 初回レンダリング時にブックマーク情報を取得
  useEffect(() => {
    fetchBookmarks();
  }, []);

  // いいね機能
  const likeMutation = useMutation({
    mutationFn: async () => {
      await TauriApi.likePost(post.id);
    },
    onSuccess: () => {
      // 楽観的UI更新
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
    },
    onError: () => {
      toast.error('いいねに失敗しました');
    },
  });

  const handleLike = () => {
    likeMutation.mutate();
  };

  const handleReply = () => {
    setShowReplyForm(!showReplyForm);
    setShowQuoteForm(false);
  };

  const handleQuote = () => {
    setShowQuoteForm(!showQuoteForm);
    setShowReplyForm(false);
  };

  // ブースト機能
  const boostMutation = useMutation({
    mutationFn: async () => {
      await TauriApi.boostPost(post.id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      toast.success('ブーストしました');
    },
    onError: () => {
      toast.error('ブーストに失敗しました');
    },
  });

  const handleBoost = () => {
    boostMutation.mutate();
  };

  // ブックマーク機能
  const bookmarkMutation = useMutation({
    mutationFn: async () => {
      await toggleBookmark(post.id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      toast.success(isPostBookmarked ? 'ブックマークを解除しました' : 'ブックマークしました');
    },
    onError: () => {
      toast.error('ブックマークの操作に失敗しました');
    },
  });

  const handleBookmark = () => {
    bookmarkMutation.mutate();
  };

  // 時間表示のフォーマット
  const timeAgo = formatDistanceToNow(new Date(post.created_at * 1000), {
    addSuffix: true,
    locale: ja,
  });

  // アバターのイニシャルを生成
  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-start gap-3">
          <Avatar>
            <AvatarImage src={post.author.picture} />
            <AvatarFallback>
              {getInitials(post.author.displayName || post.author.name || 'U')}
            </AvatarFallback>
          </Avatar>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h4 className="font-semibold">
                {post.author.displayName || post.author.name || 'ユーザー'}
              </h4>
              <span className="text-sm text-muted-foreground">{timeAgo}</span>
              {post.isSynced === false && (
                <Badge variant="outline" className="text-xs">
                  未同期
                </Badge>
              )}
            </div>
            <p className="text-sm text-muted-foreground">{post.author.npub}</p>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <p className="mb-4 whitespace-pre-wrap">{post.content}</p>
        <div className="flex items-center gap-6">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleReply}
            className={showReplyForm ? 'text-primary' : ''}
          >
            <MessageCircle className="mr-2 h-4 w-4" />
            {post.replies.length}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleBoost}
            disabled={boostMutation.isPending}
            className={post.isBoosted ? 'text-primary' : ''}
          >
            <Repeat2 className="mr-2 h-4 w-4" />
            {post.boosts || 0}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleQuote}
            className={showQuoteForm ? 'text-primary' : ''}
          >
            <Quote className="mr-2 h-4 w-4" />0
          </Button>
          <Button variant="ghost" size="sm" onClick={handleLike} disabled={likeMutation.isPending}>
            <Heart className="mr-2 h-4 w-4" />
            {post.likes}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleBookmark}
            disabled={bookmarkMutation.isPending}
            className={isPostBookmarked ? 'text-yellow-500' : ''}
          >
            <Bookmark className={`h-4 w-4 ${isPostBookmarked ? 'fill-current' : ''}`} />
          </Button>
          <ReactionPicker postId={post.id} topicId={post.topicId} />
          <Button variant="ghost" size="sm" aria-label="share" disabled>
            <Share className="h-4 w-4" />
          </Button>
        </div>

        {/* 返信フォーム */}
        <Collapsible open={showReplyForm}>
          <CollapsibleContent>
            <div className="mt-4 pt-4 border-t">
              <ReplyForm
                postId={post.id}
                topicId={post.topicId}
                onCancel={() => setShowReplyForm(false)}
                onSuccess={() => setShowReplyForm(false)}
              />
            </div>
          </CollapsibleContent>
        </Collapsible>

        {/* 引用フォーム */}
        <Collapsible open={showQuoteForm}>
          <CollapsibleContent>
            <div className="mt-4 pt-4 border-t">
              <QuoteForm
                post={post}
                onCancel={() => setShowQuoteForm(false)}
                onSuccess={() => setShowQuoteForm(false)}
              />
            </div>
          </CollapsibleContent>
        </Collapsible>
      </CardContent>
    </Card>
  );
}
