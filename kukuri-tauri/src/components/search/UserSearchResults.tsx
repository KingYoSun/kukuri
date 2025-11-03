import { useMemo, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient, type UseQueryOptions } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Loader2, UserPlus, UserCheck } from 'lucide-react';
import { Link } from '@tanstack/react-router';
import type { Profile } from '@/stores';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { TauriApi } from '@/lib/api/tauri';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import { useAuthStore } from '@/stores';
import { errorHandler } from '@/lib/errorHandler';
import { subscribeToUser } from '@/lib/api/nostr';
import { toast } from 'sonner';

type QueryOptionsWithHandlers<TData, TKey extends readonly unknown[]> = UseQueryOptions<
  TData,
  Error,
  TData,
  TKey
> & {
  onError?: (error: unknown) => void;
};

interface UserSearchResultsProps {
  query: string;
}

export function UserSearchResults({ query }: UserSearchResultsProps) {
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((state) => state.currentUser);
  const sanitizedQuery = query.trim();

  const searchResults = useQuery<Profile[], Error, Profile[], readonly ['search-users', string]>({
    queryKey: ['search-users', sanitizedQuery] as const,
    enabled: sanitizedQuery.length > 0,
    retry: false,
    queryFn: async () => {
      const profiles = await TauriApi.searchUsers(sanitizedQuery, 24);
      return profiles.map(mapUserProfileToUser);
    },
    onError: (error: unknown) => {
      errorHandler.log('UserSearchResults.searchFailed', error, {
        context: 'UserSearchResults.searchResults',
        metadata: { query: sanitizedQuery },
      });
      toast.error('ユーザー検索に失敗しました');
    },
  } as QueryOptionsWithHandlers<Profile[], readonly ['search-users', string]>);

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
        return [] as Profile[];
      }
      const profiles = await TauriApi.getFollowing(currentUser.npub);
      return profiles.map(mapUserProfileToUser);
    },
    onError: (error: unknown) => {
      errorHandler.log('UserSearchResults.followingFetchFailed', error, {
        context: 'UserSearchResults.followingQuery',
      });
      toast.error('フォロー中ユーザーの取得に失敗しました');
    },
  } as QueryOptionsWithHandlers<Profile[], readonly ['social', 'following', string | undefined]>);

  const followMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target: Profile) => {
      if (!currentUser) {
        throw new Error('ログインが必要です');
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
      if (error instanceof Error && error.message === 'ログインが必要です') {
        toast.error('フォローするにはログインが必要です');
        return;
      }
      errorHandler.log('UserSearchResults.followFailed', error, {
        context: 'UserSearchResults.followMutation',
        metadata: { targetNpub: target.npub },
      });
      toast.error('フォローに失敗しました');
    },
  });

  const unfollowMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target: Profile) => {
      if (!currentUser) {
        throw new Error('ログインが必要です');
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
      if (error instanceof Error && error.message === 'ログインが必要です') {
        toast.error('フォロー解除するにはログインが必要です');
        return;
      }
      errorHandler.log('UserSearchResults.unfollowFailed', error, {
        context: 'UserSearchResults.unfollowMutation',
        metadata: { targetNpub: target.npub },
      });
      toast.error('フォロー解除に失敗しました');
    },
  });

  const followingSet = useMemo(() => {
    const list = followingQuery.data ?? [];
    return new Set(list.map((user) => user.npub));
  }, [followingQuery.data]);

  const handleFollow = useCallback(
    (user: Profile) => {
      followMutation.mutate(user);
    },
    [followMutation],
  );

  const handleUnfollow = useCallback(
    (user: Profile) => {
      unfollowMutation.mutate(user);
    },
    [unfollowMutation],
  );

  if (!sanitizedQuery) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        検索キーワードを入力してください
      </div>
    );
  }

  if (searchResults.isLoading) {
    return (
      <div className="flex justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (searchResults.isError) {
    return (
      <div className="text-center py-12 text-sm text-muted-foreground">
        ユーザー検索でエラーが発生しました。時間をおいて再度お試しください。
      </div>
    );
  }

  const results = searchResults.data ?? [];

  if (results.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-lg font-medium">検索結果が見つかりませんでした</p>
        <p className="text-muted-foreground mt-2">
          「{sanitizedQuery}」に一致するユーザーはいません
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{results.length}人のユーザーが見つかりました</p>
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
    ? 'あなた'
    : isFollowing
      ? 'フォロー中'
      : canFollow
        ? 'フォロー'
        : 'ログインが必要';

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
