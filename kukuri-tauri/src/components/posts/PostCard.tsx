import { useEffect, useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { Bookmark, Heart, Loader2, MessageCircle, MoreVertical, Quote, Repeat2, Share, Trash2, WifiOff } from 'lucide-react';
import { toast } from 'sonner';
import { useDeletePost } from '@/hooks/usePosts';
import { useBookmarkStore, useAuthStore } from '@/stores';
import type { Post } from '@/stores';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePostStore } from '@/stores/postStore';
import { TauriApi } from '@/lib/api/tauri';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Collapsible, CollapsibleContent } from '@/components/ui/collapsible';
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
import { ReactionPicker } from './ReactionPicker';
import { QuoteForm } from './QuoteForm';
import { ReplyForm } from './ReplyForm';

interface PostCardProps {
  post: Post;
  'data-testid'?: string;
}

export function PostCard({ post, 'data-testid': dataTestId }: PostCardProps) {
  const isE2E =
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __KUKURI_E2E__?: boolean }).__KUKURI_E2E__);

  const [showReplyForm, setShowReplyForm] = useState(false);
  const [showQuoteForm, setShowQuoteForm] = useState(isE2E);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [likeCount, setLikeCount] = useState(post.likes ?? 0);
  const [boostCount, setBoostCount] = useState(post.boosts ?? 0);
  const [isBookmarkedLocal, setIsBookmarkedLocal] = useState(false);

  const queryClient = useQueryClient();
  const { isBookmarked, toggleBookmark, fetchBookmarks } = useBookmarkStore();
  const currentUser = useAuthStore((state) => state.currentUser);
  const { isOnline, pendingActions } = useOfflineStore();
  const deletePostMutation = useDeletePost();
  const likePost = usePostStore((state) => state.likePost);
  const updatePostLikesStore = usePostStore((state) => state.updatePostLikes);
  const updatePostStore = usePostStore((state) => state.updatePost);
  const canDelete = currentUser?.pubkey === post.author.pubkey;
  const replyCount =
    typeof post.replyCount === 'number'
      ? post.replyCount
      : Array.isArray(post.replies)
        ? post.replies.length
        : typeof post.replies === 'number'
          ? post.replies
          : 0;
  const baseTestId = dataTestId ?? `post-${post.id}`;
  const isPostBookmarked = isBookmarked(post.id);
  const isPostPending = pendingActions.some(
    (action) => action.actionType === 'CREATE_POST' && action.localId === post.localId,
  );

  useEffect(() => {
    setIsBookmarkedLocal(isPostBookmarked);
  }, [isPostBookmarked]);

  useEffect(() => {
    fetchBookmarks();
  }, [fetchBookmarks]);

  useEffect(() => {
    setLikeCount(post.likes ?? 0);
  }, [post.likes]);

  useEffect(() => {
    setBoostCount(post.boosts ?? 0);
  }, [post.boosts]);

  const likeMutation = useMutation({
    mutationFn: async () => {
      await likePost(post.id);
    },
  });

  const applyLikeUpdate = (nextLikes: number) => {
    setLikeCount(nextLikes);
    updatePostLikesStore(post.id, nextLikes);
    queryClient.setQueryData<Post[]>(['timeline'], (prev) =>
      prev?.map((item) => (item.id === post.id ? { ...item, likes: nextLikes } : item)) ?? prev,
    );
    if (post.topicId) {
      queryClient.setQueryData<Post[]>(['posts', post.topicId], (prev) =>
        prev?.map((item) => (item.id === post.id ? { ...item, likes: nextLikes } : item)) ?? prev,
      );
    }
  };

  const handleLike = () => {
    if (likeMutation.isPending) {
      return;
    }
    const previousLikes = likeCount ?? 0;
    const nextLikes = previousLikes + 1;
    applyLikeUpdate(nextLikes);
    if (isE2E) {
      return;
    }
    likeMutation.mutate(undefined, {
      onError: () => {
        applyLikeUpdate(previousLikes);
        toast.error('\u3044\u3044\u306d\u306b\u5931\u6557\u3057\u307e\u3057\u305f');
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

  const boostMutation = useMutation({
    mutationFn: async () => {
      await TauriApi.boostPost(post.id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      if (post.topicId) {
        queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      }
      toast.success('\u30d6\u30fc\u30b9\u30c8\u3057\u307e\u3057\u305f');
    },
  });

  const applyBoostUpdate = (nextBoosts: number, boosted: boolean) => {
    setBoostCount(nextBoosts);
    updatePostStore(post.id, { boosts: nextBoosts, isBoosted: boosted });
    queryClient.setQueryData<Post[]>(['timeline'], (prev) =>
      prev?.map((item) =>
        item.id === post.id ? { ...item, boosts: nextBoosts, isBoosted: boosted } : item,
      ) ?? prev,
    );
    if (post.topicId) {
      queryClient.setQueryData<Post[]>(['posts', post.topicId], (prev) =>
        prev?.map((item) =>
          item.id === post.id ? { ...item, boosts: nextBoosts, isBoosted: boosted } : item,
        ) ?? prev,
      );
    }
  };

  const handleBoost = () => {
    if (boostMutation.isPending) {
      return;
    }
    const previousBoosts = boostCount ?? 0;
    const nextBoosts = previousBoosts + 1;
    applyBoostUpdate(nextBoosts, true);
    if (isE2E) {
      return;
    }
    boostMutation.mutate(undefined, {
      onError: () => {
        applyBoostUpdate(previousBoosts, post.isBoosted ?? false);
        toast.error('\u30d6\u30fc\u30b9\u30c8\u306b\u5931\u6557\u3057\u307e\u3057\u305f');
      },
    });
  };

  const bookmarkMutation = useMutation({
    mutationFn: async () => {
      await toggleBookmark(post.id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      toast.success(isPostBookmarked ? '\u30d6\u30c3\u30af\u30de\u30fc\u30af\u3092\u89e3\u9664\u3057\u307e\u3057\u305f' : '\u30d6\u30c3\u30af\u30de\u30fc\u30af\u3057\u307e\u3057\u305f');
    },
    onError: () => {
      toast.error('\u30d6\u30c3\u30af\u30de\u30fc\u30af\u306e\u64cd\u4f5c\u306b\u5931\u6557\u3057\u307e\u3057\u305f');
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

  const timeAgo = formatDistanceToNow(new Date(post.created_at * 1000), {
    addSuffix: true,
    locale: ja,
  });

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
                  {post.author.displayName || post.author.name || '\u30e6\u30fc\u30b6\u30fc'}
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
                        \u30aa\u30d5\u30e9\u30a4\u30f3\u4fdd\u5b58
                      </>
                    ) : (
                      <>
                        <div className="h-2 w-2 rounded-full bg-yellow-500 animate-pulse" />
                        \u540c\u671f\u5f85\u3061
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
                <Button variant="ghost" size="icon" aria-label="\u6295\u7a3f\u30e1\u30cb\u30e5\u30fc">
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  className="text-destructive focus:text-destructive"
                  onClick={() => setShowDeleteDialog(true)}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  \u524a\u9664
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
            <AlertDialogTitle>\u6295\u7a3f\u3092\u524a\u9664\u3057\u307e\u3059\u304b\uff1f</AlertDialogTitle>
            <AlertDialogDescription>
              \u4e00\u5ea6\u524a\u9664\u3059\u308b\u3068\u3053\u306e\u6295\u7a3f\u306f\u5fa9\u5143\u3067\u304d\u307e\u305b\u3093\u3002\u3088\u308d\u3057\u3051\u308c\u3070\u300c\u524a\u9664\u3059\u308b\u300d\u3092\u62bc\u3057\u3066\u304f\u3060\u3055\u3044\u3002
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deletePostMutation.isPending}>
              \u30ad\u30e3\u30f3\u30bb\u30eb
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
              \u524a\u9664\u3059\u308b
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
