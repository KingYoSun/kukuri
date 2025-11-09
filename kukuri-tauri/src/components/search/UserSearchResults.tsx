import { useMemo } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { Loader2, UserCheck, UserPlus } from 'lucide-react';
import { toast } from 'sonner';

import { Card, CardContent } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import type { Profile } from '@/stores';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { TauriApi } from '@/lib/api/tauri';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import { useAuthStore } from '@/stores';
import { errorHandler } from '@/lib/errorHandler';
import { subscribeToUser } from '@/lib/api/nostr';
import { SearchErrorState } from '@/components/search/SearchErrorState';
import { useUserSearchQuery } from '@/hooks/useUserSearchQuery';

interface UserSearchResultsProps {
  query: string;
}

export function UserSearchResults({ query }: UserSearchResultsProps) {
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((state) => state.currentUser);

  const {
    status,
    sanitizedQuery,
    results,
    totalCount,
    tookMs,
    hasNextPage,
    isFetching,
    isFetchingNextPage,
    fetchNextPage,
    errorKey,
    retryAfterSeconds,
    onRetry,
  } = useUserSearchQuery(query, {
    viewerNpub: currentUser?.npub ?? null,
    pageSize: 24,
  });

  const followingQuery = useQuery<
    Profile[],
    Error,
    Profile[],
    readonly ['social', 'following', string | undefined]
  >({
    queryKey: ['social', 'following', currentUser?.npub] as const,
    enabled: Boolean(currentUser),
    retry: false,
    queryFn: async () => {
      if (!currentUser) {
        return [];
      }
      const page = await TauriApi.getFollowing({
        npub: currentUser.npub,
      });
      return page.items.map(mapUserProfileToUser);
    },
  });

  const followingSet = useMemo(
    () => new Set(followingQuery.data?.map((profile) => profile.npub)),
    [followingQuery.data],
  );

  const followMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target) => {
      if (!currentUser) {
        throw new Error('ログインしてから操作してください');
      }
      await TauriApi.followUser(currentUser.npub, target.npub);
      if (target.pubkey) {
        await subscribeToUser(target.pubkey);
      }
    },
    onSuccess: (_, target) => {
      queryClient.setQueryData<Profile[] | undefined>(
        ['social', 'following', currentUser?.npub],
        (prev = []) => {
          if (prev.some((profile) => profile.npub === target.npub)) {
            return prev;
          }
          return [...prev, target];
        },
      );
      toast.success(`${target.displayName || target.npub} をフォローしました`);
    },
    onError: (error, target) => {
      const message =
        error instanceof Error && error.message === 'ログインしてから操作してください'
          ? 'ログインが必要です'
          : 'フォローに失敗しました';
      toast.error(message);
      errorHandler.log('UserSearch.follow_failed', error, {
        context: 'UserSearchResults.followMutation',
        metadata: { targetNpub: target.npub },
      });
    },
  });

  const unfollowMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target) => {
      if (!currentUser) {
        throw new Error('ログインしてから操作してください');
      }
      await TauriApi.unfollowUser(currentUser.npub, target.npub);
    },
    onSuccess: (_, target) => {
      queryClient.setQueryData<Profile[] | undefined>(
        ['social', 'following', currentUser?.npub],
        (prev = []) => prev.filter((profile) => profile.npub !== target.npub),
      );
      toast.success(`${target.displayName || target.npub} のフォローを解除しました`);
    },
    onError: (error, target) => {
      const message =
        error instanceof Error && error.message === 'ログインしてから操作してください'
          ? 'ログインが必要です'
          : 'フォロー解除に失敗しました';
      toast.error(message);
      errorHandler.log('UserSearch.unfollow_failed', error, {
        context: 'UserSearchResults.unfollowMutation',
        metadata: { targetNpub: target.npub },
      });
    },
  });

  const handleFollow = (target: Profile) => {
    followMutation.mutate(target);
  };

  const handleUnfollow = (target: Profile) => {
    unfollowMutation.mutate(target);
  };

  const showIdle = status === 'idle';
  const showReady = status === 'ready';
  const showEmpty = status === 'empty';
  const showInitialLoading = status === 'loading' && results.length === 0;

  return (
    <div className="space-y-4">
      {showIdle && (
        <p className="text-sm text-muted-foreground">検索キーワードを入力してください。</p>
      )}

      {showReady && (
        <p className="text-sm text-muted-foreground">入力を確定すると検索が始まります。</p>
      )}

      {errorKey && (
        <SearchErrorState
          errorKey={errorKey}
          retryAfterSeconds={retryAfterSeconds ?? undefined}
          onRetry={onRetry}
        />
      )}

      {showInitialLoading && (
        <div className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span>検索しています...</span>
        </div>
      )}

      {showEmpty && (
        <div className="text-center py-12">
          <p className="text-lg font-medium">該当するユーザーが見つかりませんでした</p>
          {sanitizedQuery && (
            <p className="text-muted-foreground mt-2">
              「{sanitizedQuery}」に一致するユーザーはいません。
            </p>
          )}
        </div>
      )}

      {results.length > 0 && (
        <>
          <div className="flex items-center justify-between text-sm text-muted-foreground">
            <span>{totalCount.toLocaleString()} 件ヒット</span>
            {tookMs > 0 && <span>{tookMs} ms</span>}
          </div>

          <div className="space-y-4">
            {results.map((user) => (
              <UserCard
                key={user.npub || user.id}
                user={user}
                isFollowing={followingSet.has(user.npub)}
                isSelf={currentUser?.npub === user.npub}
                isProcessing={
                  (followMutation.isPending && followMutation.variables?.npub === user.npub) ||
                  (unfollowMutation.isPending && unfollowMutation.variables?.npub === user.npub)
                }
                canFollow={Boolean(currentUser)}
                onFollow={() => handleFollow(user)}
                onUnfollow={() => handleUnfollow(user)}
              />
            ))}
          </div>

          {hasNextPage && (
            <div className="flex justify-center pt-2">
              <Button variant="outline" onClick={() => fetchNextPage()} disabled={isFetchingNextPage}>
                {isFetchingNextPage ? '読み込み中...' : 'さらに表示'}
              </Button>
            </div>
          )}
        </>
      )}

      {isFetching && results.length > 0 && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span>更新中...</span>
        </div>
      )}
    </div>
  );
}

interface UserCardProps {
  user: Profile;
  isFollowing: boolean;
  isSelf: boolean;
  isProcessing: boolean;
  canFollow: boolean;
  onFollow: () => void;
  onUnfollow: () => void;
}

function UserCard({
  user,
  isFollowing,
  isSelf,
  isProcessing,
  canFollow,
  onFollow,
  onUnfollow,
}: UserCardProps) {
  const avatarSrc = resolveUserAvatarSrc(user);
  const initials = getInitials(user.displayName || user.name || 'U');
  const followLabel = isSelf
    ? '自分です'
    : isFollowing
      ? 'フォロー中'
      : canFollow
        ? 'フォロー'
        : 'ログインが必要です';

  const handleClick = () => {
    if (isProcessing || isSelf || !canFollow) {
      return;
    }
    if (isFollowing) {
      onUnfollow();
    } else {
      onFollow();
    }
  };

  return (
    <Card>
      <CardContent className="flex items-center justify-between p-4">
        <div className="flex items-center gap-3">
          <Avatar className="h-12 w-12">
            <AvatarImage src={avatarSrc} />
            <AvatarFallback>{initials}</AvatarFallback>
          </Avatar>
          <div>
            <Link
              to="/profile/$userId"
              params={{ userId: user.npub || user.id }}
              className="font-semibold hover:underline"
            >
              {user.displayName || user.name || 'ユーザー'}
            </Link>
            {user.nip05 && <p className="text-sm text-muted-foreground">{user.nip05}</p>}
            {user.about && (
              <p className="text-sm text-muted-foreground mt-1 line-clamp-1">{user.about}</p>
            )}
            <p className="text-xs text-muted-foreground mt-1 break-all">{user.npub}</p>
          </div>
        </div>
        <Button
          size="sm"
          variant={isFollowing ? 'secondary' : 'outline'}
          disabled={!canFollow || isSelf || isProcessing}
          onClick={handleClick}
          className="min-w-[110px]"
        >
          {isProcessing ? (
            <Loader2 className="h-4 w-4 mr-2 animate-spin" />
          ) : isFollowing ? (
            <UserCheck className="h-4 w-4 mr-2" />
          ) : (
            <UserPlus className="h-4 w-4 mr-2" />
          )}
          {followLabel}
        </Button>
      </CardContent>
    </Card>
  );
}

function getInitials(name: string) {
  return name
    .split(' ')
    .map((n) => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
}
