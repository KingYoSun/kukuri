import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import {
  Bookmark,
  Flag,
  Heart,
  Lock,
  Loader2,
  MessageCircle,
  MoreVertical,
  Quote,
  Repeat2,
  Share,
  ShieldCheck,
  Trash2,
  WifiOff,
} from 'lucide-react';
import { toast } from 'sonner';
import { useDeletePost } from '@/hooks/usePosts';
import { useBookmarkStore, useAuthStore } from '@/stores';
import type { Post } from '@/stores';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePostStore } from '@/stores/postStore';
import { TauriApi } from '@/lib/api/tauri';
import { communityNodeApi } from '@/lib/api/communityNode';
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ReactionPicker } from './ReactionPicker';
import { QuoteForm } from './QuoteForm';
import { ReplyForm } from './ReplyForm';
import { errorHandler } from '@/lib/errorHandler';

interface PostCardProps {
  post: Post;
  'data-testid'?: string;
}

const scopeLabels: Record<string, string> = {
  public: '公開',
  friend_plus: 'フレンド+',
  friend: 'フレンド',
  invite: '招待',
};

const reportReasonOptions = [
  { value: 'spam', label: 'スパム' },
  { value: 'harassment', label: '嫌がらせ' },
  { value: 'hate', label: 'ヘイト' },
  { value: 'scam', label: '詐欺' },
  { value: 'nsfw', label: 'センシティブ' },
  { value: 'illegal', label: '違法' },
  { value: 'other', label: 'その他' },
];

const defaultReportReason = reportReasonOptions[0]?.value ?? 'spam';

const formatScopeLabel = (scope?: string) => {
  if (!scope) {
    return scopeLabels.public;
  }
  return scopeLabels[scope] ?? scope;
};

const shortenPubkey = (value: string) =>
  value.length > 16 ? `${value.slice(0, 8)}...${value.slice(-4)}` : value;

const formatAttesterLabel = (baseUrl?: string, pubkey?: string | null) => {
  if (baseUrl) {
    try {
      return new URL(baseUrl).host;
    } catch {
      return baseUrl;
    }
  }
  if (pubkey) {
    return shortenPubkey(pubkey);
  }
  return null;
};

const extractTagValue = (event: unknown, tagName: string): string | null => {
  if (!event || typeof event !== 'object') {
    return null;
  }
  const tags = (event as { tags?: unknown }).tags;
  if (!Array.isArray(tags)) {
    return null;
  }
  for (const tag of tags) {
    if (Array.isArray(tag) && tag[0] === tagName && typeof tag[1] === 'string') {
      return tag[1];
    }
  }
  return null;
};

const formatLabelSummary = (event: unknown): string | null => {
  const label = extractTagValue(event, 'label');
  if (!label) {
    return null;
  }
  const confidence = extractTagValue(event, 'confidence');
  if (!confidence) {
    return label;
  }
  const numeric = Number(confidence);
  if (Number.isFinite(numeric)) {
    return `${label} (${numeric.toFixed(2)})`;
  }
  return label;
};

const parseLabelSummaries = (payload: unknown): string[] => {
  if (!payload || typeof payload !== 'object') {
    return [];
  }
  const items = (payload as { items?: unknown }).items;
  if (!Array.isArray(items)) {
    return [];
  }
  const summaries = items
    .map((item) => formatLabelSummary(item))
    .filter((value): value is string => Boolean(value));
  return Array.from(new Set(summaries));
};

const toNumber = (value: unknown): number | null => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  return null;
};

export function PostCard({ post, 'data-testid': dataTestId }: PostCardProps) {
  const [showReplyForm, setShowReplyForm] = useState(false);
  const [showQuoteForm, setShowQuoteForm] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showReportDialog, setShowReportDialog] = useState(false);
  const [likeCount, setLikeCount] = useState(post.likes ?? 0);
  const [boostCount, setBoostCount] = useState(post.boosts ?? 0);
  const [isBookmarkedLocal, setIsBookmarkedLocal] = useState(false);
  const [reportReason, setReportReason] = useState(defaultReportReason);

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
  const communityConfigQuery = useQuery({
    queryKey: ['community-node', 'config'],
    queryFn: () => communityNodeApi.getConfig(),
    staleTime: 1000 * 60 * 5,
  });
  const trustAnchorQuery = useQuery({
    queryKey: ['community-node', 'trust-anchor'],
    queryFn: () => communityNodeApi.getTrustAnchor(),
    staleTime: 1000 * 60 * 5,
  });
  const labelNodes =
    communityConfigQuery.data?.nodes?.filter((node) => node.roles.labels && node.has_token) ?? [];
  const trustNodes =
    communityConfigQuery.data?.nodes?.filter((node) => node.roles.trust && node.has_token) ?? [];
  const reportNode = communityConfigQuery.data?.nodes?.find((node) => node.has_token) ?? null;
  const trustAnchorAttester = trustAnchorQuery.data?.attester ?? null;
  const trustAnchorNode = trustAnchorAttester
    ? (trustNodes.find((node) => node.pubkey === trustAnchorAttester) ?? null)
    : null;
  const trustBaseUrl = trustAnchorNode?.base_url;
  const trustAttesterLabel = formatAttesterLabel(trustBaseUrl, trustAnchorAttester);
  const enableLabels = labelNodes.length > 0;
  const enableTrust = trustNodes.length > 0;
  const canReport = Boolean(reportNode);
  const labelQuery = useQuery({
    queryKey: ['community-node', 'labels', post.id],
    queryFn: () =>
      communityNodeApi.listLabels({
        target: `event:${post.id}`,
        limit: 10,
      }),
    enabled: enableLabels,
    staleTime: 1000 * 60 * 5,
  });
  const trustSubject = `pubkey:${post.author.pubkey}`;
  const reportTarget = `event:${post.id}`;
  const trustReportQuery = useQuery({
    queryKey: [
      'community-node',
      'trust',
      'report-based',
      post.author.pubkey,
      trustAnchorAttester ?? 'auto',
    ],
    queryFn: () =>
      communityNodeApi.trustReportBased({ subject: trustSubject, base_url: trustBaseUrl }),
    enabled: enableTrust,
    staleTime: 1000 * 60 * 5,
  });
  const trustDensityQuery = useQuery({
    queryKey: [
      'community-node',
      'trust',
      'communication-density',
      post.author.pubkey,
      trustAnchorAttester ?? 'auto',
    ],
    queryFn: () =>
      communityNodeApi.trustCommunicationDensity({ subject: trustSubject, base_url: trustBaseUrl }),
    enabled: enableTrust,
    staleTime: 1000 * 60 * 5,
  });
  const labelSummaries = parseLabelSummaries(labelQuery.data);
  const reportScore = toNumber(trustReportQuery.data?.score);
  const densityScore = toNumber(trustDensityQuery.data?.score);
  const showAttesterLabel = Boolean(trustAnchorAttester && trustAttesterLabel);
  const resolvedScope = post.scope ?? 'public';
  const showScopeBadge = resolvedScope !== 'public';
  const showEncryptedBadge = post.isEncrypted === true;
  const showPostMenu = canDelete || canReport;

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

  useEffect(() => {
    if (labelQuery.isError) {
      errorHandler.log('Failed to load community node labels', labelQuery.error, {
        context: 'PostCard.labels',
        metadata: { postId: post.id },
      });
    }
  }, [labelQuery.isError, labelQuery.error, post.id]);

  useEffect(() => {
    if (trustAnchorQuery.isError) {
      errorHandler.log('Failed to load community node trust anchor', trustAnchorQuery.error, {
        context: 'PostCard.trustAnchor',
      });
    }
  }, [trustAnchorQuery.isError, trustAnchorQuery.error]);

  useEffect(() => {
    if (trustReportQuery.isError) {
      errorHandler.log('Failed to load community node trust score', trustReportQuery.error, {
        context: 'PostCard.trustReport',
        metadata: { author: post.author.pubkey },
      });
    }
    if (trustDensityQuery.isError) {
      errorHandler.log('Failed to load community node trust density', trustDensityQuery.error, {
        context: 'PostCard.trustDensity',
        metadata: { author: post.author.pubkey },
      });
    }
  }, [
    trustReportQuery.isError,
    trustReportQuery.error,
    trustDensityQuery.isError,
    trustDensityQuery.error,
    post.author.pubkey,
  ]);

  const reportMutation = useMutation({
    mutationFn: async () => {
      if (!reportNode) {
        throw new Error('Community node is not configured');
      }
      if (!reportReason) {
        throw new Error('Report reason is required');
      }
      return await communityNodeApi.submitReport({
        base_url: reportNode.base_url,
        target: reportTarget,
        reason: reportReason,
      });
    },
    onSuccess: () => {
      toast.success('通報を受け付けました');
      setShowReportDialog(false);
      setReportReason(defaultReportReason);
    },
    onError: (error) => {
      errorHandler.log('Failed to submit community node report', error, {
        context: 'PostCard.report',
        showToast: true,
        toastTitle: '通報に失敗しました',
      });
    },
  });

  const handleReportDialogChange = (open: boolean) => {
    if (reportMutation.isPending) {
      return;
    }
    if (!open) {
      setReportReason(defaultReportReason);
    }
    setShowReportDialog(open);
  };

  const handleOpenReportDialog = () => {
    if (!canReport) {
      toast.error('Community Node が未設定です');
      return;
    }
    setShowReportDialog(true);
  };

  const handleSubmitReport = () => {
    if (reportMutation.isPending) {
      return;
    }
    reportMutation.mutate();
  };

  const likeMutation = useMutation({
    mutationFn: async () => {
      await likePost(post.id);
    },
  });

  const applyLikeUpdate = (nextLikes: number) => {
    setLikeCount(nextLikes);
    updatePostLikesStore(post.id, nextLikes);
    queryClient.setQueryData<Post[]>(
      ['timeline'],
      (prev) =>
        prev?.map((item) => (item.id === post.id ? { ...item, likes: nextLikes } : item)) ?? prev,
    );
    if (post.topicId) {
      queryClient.setQueryData<Post[]>(
        ['posts', post.topicId],
        (prev) =>
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
    likeMutation.mutate(undefined, {
      onError: () => {
        applyLikeUpdate(previousLikes);
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

  const boostMutation = useMutation({
    mutationFn: async () => {
      await TauriApi.boostPost(post.id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      if (post.topicId) {
        queryClient.invalidateQueries({ queryKey: ['posts', post.topicId] });
      }
      toast.success('ブーストしました');
    },
  });

  const applyBoostUpdate = (nextBoosts: number, boosted: boolean) => {
    setBoostCount(nextBoosts);
    updatePostStore(post.id, { boosts: nextBoosts, isBoosted: boosted });
    queryClient.setQueryData<Post[]>(
      ['timeline'],
      (prev) =>
        prev?.map((item) =>
          item.id === post.id ? { ...item, boosts: nextBoosts, isBoosted: boosted } : item,
        ) ?? prev,
    );
    if (post.topicId) {
      queryClient.setQueryData<Post[]>(
        ['posts', post.topicId],
        (prev) =>
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
    boostMutation.mutate(undefined, {
      onError: () => {
        applyBoostUpdate(previousBoosts, post.isBoosted ?? false);
        toast.error('ブーストに失敗しました');
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
                    data-testid={`${baseTestId}-sync-badge`}
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
          {showPostMenu && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="投稿メニュー"
                  data-testid={`${baseTestId}-menu`}
                >
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {canReport && (
                  <DropdownMenuItem
                    onClick={handleOpenReportDialog}
                    data-testid={`${baseTestId}-report`}
                  >
                    <Flag className="mr-2 h-4 w-4" />
                    通報
                  </DropdownMenuItem>
                )}
                {canDelete && (
                  <DropdownMenuItem
                    className="text-destructive focus:text-destructive"
                    onClick={() => setShowDeleteDialog(true)}
                    data-testid={`${baseTestId}-delete`}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    削除
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {(showScopeBadge ||
          showEncryptedBadge ||
          labelSummaries.length > 0 ||
          reportScore !== null ||
          densityScore !== null) && (
          <div className="mb-3 flex flex-wrap items-center gap-2 text-xs">
            {showScopeBadge && (
              <Badge
                variant="outline"
                data-testid={`${baseTestId}-scope`}
                data-scope={resolvedScope}
              >
                {formatScopeLabel(resolvedScope)}
              </Badge>
            )}
            {showEncryptedBadge && (
              <Badge
                variant="secondary"
                className="flex items-center gap-1"
                data-testid={`${baseTestId}-encrypted`}
              >
                <Lock className="h-3 w-3" />
                暗号化
              </Badge>
            )}
            {labelSummaries.slice(0, 3).map((label, index) => (
              <Badge
                key={`${label}-${index}`}
                variant="outline"
                data-testid={`${baseTestId}-label-${index}`}
                data-label={label}
              >
                ラベル: {label}
              </Badge>
            ))}
            {reportScore !== null && (
              <Badge
                variant="secondary"
                className="flex items-center gap-1"
                data-testid={`${baseTestId}-trust-report`}
                data-score={reportScore.toFixed(2)}
                data-attester={showAttesterLabel ? (trustAttesterLabel ?? '') : ''}
              >
                <ShieldCheck className="h-3 w-3" />
                信頼 {reportScore.toFixed(2)}
                {showAttesterLabel && (
                  <span className="text-[10px] text-muted-foreground">({trustAttesterLabel})</span>
                )}
              </Badge>
            )}
            {densityScore !== null && (
              <Badge
                variant="secondary"
                className="flex items-center gap-1"
                data-testid={`${baseTestId}-trust-density`}
                data-score={densityScore.toFixed(2)}
                data-attester={showAttesterLabel ? (trustAttesterLabel ?? '') : ''}
              >
                <ShieldCheck className="h-3 w-3" />
                通信 {densityScore.toFixed(2)}
                {showAttesterLabel && (
                  <span className="text-[10px] text-muted-foreground">({trustAttesterLabel})</span>
                )}
              </Badge>
            )}
          </div>
        )}
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
                scope={post.scope}
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
      <Dialog open={showReportDialog} onOpenChange={handleReportDialogChange}>
        <DialogContent data-testid={`${baseTestId}-report-dialog`}>
          <DialogHeader>
            <DialogTitle>通報</DialogTitle>
            <DialogDescription>Community Node に通報理由を送信します。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="rounded-md border p-3 text-xs text-muted-foreground">
              <p className="font-medium text-foreground">
                対象: {post.author.displayName || post.author.name || 'ユーザー'}
              </p>
              <p className="break-all">event: {post.id}</p>
            </div>
            <div className="space-y-2">
              <Label htmlFor={`${baseTestId}-report-reason`}>理由</Label>
              <Select value={reportReason} onValueChange={setReportReason}>
                <SelectTrigger
                  id={`${baseTestId}-report-reason`}
                  data-testid={`${baseTestId}-report-reason`}
                >
                  <SelectValue placeholder="理由を選択" />
                </SelectTrigger>
                <SelectContent>
                  {reportReasonOptions.map((option) => (
                    <SelectItem
                      key={option.value}
                      value={option.value}
                      data-testid={`${baseTestId}-report-reason-${option.value}`}
                    >
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="ghost"
              onClick={() => handleReportDialogChange(false)}
              disabled={reportMutation.isPending}
              data-testid={`${baseTestId}-report-cancel`}
            >
              キャンセル
            </Button>
            <Button
              type="button"
              onClick={handleSubmitReport}
              disabled={reportMutation.isPending || !reportReason}
              data-testid={`${baseTestId}-report-submit`}
            >
              {reportMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  送信中...
                </>
              ) : (
                '通報する'
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle data-testid={`${baseTestId}-confirm-title`}>
              投稿を削除しますか？
            </AlertDialogTitle>
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
              data-testid={`${baseTestId}-confirm-delete`}
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
