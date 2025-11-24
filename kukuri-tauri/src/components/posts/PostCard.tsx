import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import {
  WifiOff,
  Heart,
  MessageCircle,
  Repeat2,
  Share,
  Bookmark,
  Quote,
  MoreVertical,
  Trash2,
  Loader2,
} from 'lucide-react';
import { useOfflineStore } from '@/stores/offlineStore';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import type { Post } from '@/stores';
import { useBookmarkStore, useAuthStore } from '@/stores';
import { usePostStore } from '@/stores/postStore';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';
import { ReplyForm } from './ReplyForm';
import { QuoteForm } from './QuoteForm';
import { ReactionPicker } from './ReactionPicker';
import { Collapsible, CollapsibleContent } from '@/components/ui/collapsible';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { useDeletePost } from '@/hooks/usePosts';

interface PostCardProps {
  post: Post;
  'data-testid'?: string;
}

export function PostCard({ post, 'data-testid': dataTestId }: PostCardProps) {
  const [showReplyForm, setShowReplyForm] = useState(false);
  const [showQuoteForm, setShowQuoteForm] = useState(false);
  const queryClient = useQueryClient();
  const { isBookmarked, toggleBookmark, fetchBookmarks } = useBookmarkStore();
  const currentUser = useAuthStore((state) => state.currentUser);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const isPostBookmarked = isBookmarked(post.id);
  const [isBookmarkedLocal, setIsBookmarkedLocal] = useState(isPostBookmarked);
  const { isOnline, pendingActions } = useOfflineStore();
  const canDelete = currentUser?.pubkey === post.author.pubkey;
  const deletePostMutation = useDeletePost();
  const replyCount =
    typeof post.replyCount === 'number'
      ? post.replyCount
      : Array.isArray(post.replies)
        ? post.replies.length
        : typeof post.replies === 'number'
          ? post.replies
          : 0;
  const baseTestId = dataTestId ?? `post-${post.id}`;
  const [likeCount, setLikeCount] = useState(post.likes ?? 0);
  const [boostCount, setBoostCount] = useState(post.boosts ?? 0);

  useEffect(() => {
    setLikeCount(post.likes ?? 0);
  }, [post.likes]);

  useEffect(() => {
    setBoostCount(post.boosts ?? 0);
  }, [post.boosts]);

  // この投稿が未同期かどうかを確認
  const isPostPending = pendingActions.some(
    (action) => action.actionType === 'CREATE_POST' && action.localId === post.localId,
  );

  // 初回レンダリング時にブックマーク情報を取得
  useEffect(() => {
    fetchBookmarks();
  }, [fetchBookmarks]);

  useEffect(() => {
    setIsBookmarkedLocal(isPostBookmarked);
  }, [isPostBookmarked]);

  // いいね機能
  const likePost = usePostStore((state) => state.likePost);
  const likeMutation = useMutation({
    mutationFn: async () => {
      await likePost(post.id);
    },
  });

  const handleLike = () => {
    if (likeMutation.isPending) {
      return;
    }
    const previousLikes = likeCount ?? 0;
    setLikeCount(previousLikes + 1);
    likeMutation.mutate(undefined, {
      onError: () => {
        setLikeCount(previousLikes);
        toast.error('いいねに失敗しました');
      },
    });
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
      const updateBoosts = (posts?: Post[]) =>
        posts?.map((item) =>
          item.id === post.id ? { ...item, boosts: (item.boosts ?? 0) + 1, isBoosted: true } : item,
        ) ?? posts;
      queryClient.setQueryData<Post[]>(['timeline'], (prev) => updateBoosts(prev));
      queryClient.setQueryData<Post[]>(['posts', post.topicId], (prev) => updateBoosts(prev));
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      toast.success('ブーストしました');
    },
  });

  const handleBoost = () => {
    if (boostMutation.isPending) {
      return;
    }
    const previousBoosts = boostCount ?? 0;
    setBoostCount(previousBoosts + 1);
    boostMutation.mutate(undefined, {
      onError: () => {
        setBoostCount(previousBoosts);
        toast.error('ブーストに失敗しました');
      },
    });
  };

  // Bookmark
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
    if (bookmarkMutation.isPending) {
      return;
    }
    setIsBookmarkedLocal((prev) => !prev);
    bookmarkMutation.mutate(undefined, {
      onError: () => {
        setIsBookmarkedLocal(isPostBookmarked);
      },
    });
  };

  const handleConfirmDelete = () => {
    deletePostMutation.mutate(post, {
      onSettled: () => setShowDeleteDialog(false),
    });
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

  const authorAvatarSrc = resolveUserAvatarSrc(post.author);

  return (
    <Card data-testid={baseTestId}>
      <CardHeader>
        <div className="flex items-start justify-between gap-3">
          <div className="flex flex-1 items-start gap-3">
            <Avatar>
              <AvatarImage src={authorAvatarSrc} />
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
                {(post.isSynced === false || isPostPending) && (
                  <Badge
                    variant="outline"
                    className={`text-xs flex items-center gap-1 ${
                      !isOnline
                        ? 'border-orange-500 text-orange-600 dark:text-orange-400'
                        : 'border-yellow-500 text-yellow-600 dark:text-yellow-400'
                    }`}
                  >
                    {!isOnline ? (
                      <>
                        <WifiOff className="h-3 w-3" />
                        オフライン保存
                      </>
                    ) : (
                      <>
                        <div className="h-2 w-2 rounded-full bg-yellow-500 animate-pulse" />
                        同期待ち
                      </>
                    )}
                  </Badge>
                )}
              </div>
              <p className="text-sm text-muted-foreground">{post.author.npub}</p>
            </div>
          </div>
          {canDelete && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" aria-label="投稿メニュー">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  className="text-destructive focus:text-destructive"
                  onClick={() => setShowDeleteDialog(true)}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  削除
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <p className="mb-4 whitespace-pre-wrap">{post.content}</p>
        <div className="flex items-center gap-6">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleReply}
            data-testid={`${baseTestId}-reply`}
            className={showReplyForm ? 'text-primary' : ''}
          >
            <MessageCircle className="mr-2 h-4 w-4" />
            {replyCount}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleBoost}
            disabled={boostMutation.isPending}
            data-testid={`${baseTestId}-boost`}
            className={post.isBoosted || boostCount > (post.boosts ?? 0) ? 'text-primary' : ''}
          >
            <Repeat2 className="mr-2 h-4 w-4" />
            {boostCount}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleQuote}
            data-testid={`${baseTestId}-quote`}
            className={showQuoteForm ? 'text-primary' : ''}
          >
            <Quote className="mr-2 h-4 w-4" />0
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleLike}
            disabled={likeMutation.isPending}
            data-testid={`${baseTestId}-like`}
          >
            <Heart className="mr-2 h-4 w-4" />
            {likeCount}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleBookmark}
            disabled={bookmarkMutation.isPending}
            className={isBookmarkedLocal ? 'text-yellow-500' : ''}
            data-testid={`${baseTestId}-bookmark`}
            aria-pressed={isBookmarkedLocal}
          >
            <Bookmark className={`h-4 w-4 ${isBookmarkedLocal ? 'fill-current' : ''}`} />
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
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>投稿を削除しますか？</AlertDialogTitle>
            <AlertDialogDescription>
              一度削除するとこの投稿は復元できません。よろしければ「削除する」を押してください。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deletePostMutation.isPending}>
              キャンセル
            </AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={handleConfirmDelete}
              disabled={deletePostMutation.isPending}
            >
              {deletePostMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="mr-2 h-4 w-4" />
              )}
              削除する
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
