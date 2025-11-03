import { useMemo, useCallback } from 'react';
import { createFileRoute, Link } from '@tanstack/react-router';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Avatar,
  AvatarFallback,
  AvatarImage,
} from '@/components/ui/avatar';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Loader2, Copy, ArrowLeft, UserPlus, UserCheck, MessageCircle } from 'lucide-react';
import { TauriApi } from '@/lib/api/tauri';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import type { Post, Profile } from '@/stores';
import { PostCard } from '@/components/posts/PostCard';
import { toast } from 'sonner';
import { useAuthStore } from '@/stores';
import { errorHandler } from '@/lib/errorHandler';
import { subscribeToUser } from '@/lib/api/nostr';

export const Route = createFileRoute('/profile/$userId')({
  component: ProfilePage,
});

function ProfilePage() {
  const { userId } = Route.useParams();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((state) => state.currentUser);

  const profileQuery = useQuery({
    queryKey: ['userProfile', userId],
    queryFn: async () => {
      const byNpub = await TauriApi.getUserProfile(userId);
      if (byNpub) {
        return mapUserProfileToUser(byNpub);
      }
      const byPubkey = await TauriApi.getUserProfileByPubkey(userId);
      if (byPubkey) {
        return mapUserProfileToUser(byPubkey);
      }
      return null;
    },
  });

  const profile = profileQuery.data;

  const postsQuery = useQuery({
    queryKey: ['userPosts', profile?.pubkey],
    enabled: Boolean(profile),
    queryFn: async () => {
      if (!profile) return [] as Post[];
      const apiPosts = await TauriApi.getPosts({
        author_pubkey: profile.pubkey,
        pagination: { limit: 50 },
      });
      return Promise.all(apiPosts.map((post) => mapPostResponseToDomain(post)));
    },
  });

  const followersQuery = useQuery({
    queryKey: ['profile', profile?.npub ?? userId, 'followers'],
    enabled: Boolean(profile),
    retry: false,
    queryFn: async () => {
      if (!profile) {
        return [] as Profile[];
      }
      const response = await TauriApi.getFollowers(profile.npub);
      return response.map(mapUserProfileToUser);
    },
    onError: (error) => {
      errorHandler.log('ProfilePage.followersFetchFailed', error, {
        context: 'ProfilePage.followersQuery',
        userId,
      });
      toast.error('フォロワーの取得に失敗しました');
    },
  });

  const followingQuery = useQuery({
    queryKey: ['profile', profile?.npub ?? userId, 'following'],
    enabled: Boolean(profile),
    retry: false,
    queryFn: async () => {
      if (!profile) {
        return [] as Profile[];
      }
      const response = await TauriApi.getFollowing(profile.npub);
      return response.map(mapUserProfileToUser);
    },
    onError: (error) => {
      errorHandler.log('ProfilePage.followingFetchFailed', error, {
        context: 'ProfilePage.followingQuery',
        userId,
      });
      toast.error('フォロー中ユーザーの取得に失敗しました');
    },
  });

  const posts = postsQuery.data ?? [];
  const followers = followersQuery.data ?? [];
  const following = followingQuery.data ?? [];

  const isCurrentUser = Boolean(profile && currentUser?.npub === profile.npub);
  const canFollow = Boolean(currentUser) && Boolean(profile) && !isCurrentUser;

  const isFollowing = useMemo(() => {
    if (!profile || !currentUser || isCurrentUser) {
      return false;
    }
    return followers.some((follower) => follower.npub === currentUser.npub);
  }, [followers, currentUser, profile, isCurrentUser]);

  const followMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target) => {
      if (!currentUser) {
        throw new Error('ログインが必要です');
      }
      await TauriApi.followUser(currentUser.npub, target.npub);
      if (target.pubkey) {
        try {
          await subscribeToUser(target.pubkey);
        } catch (error) {
          errorHandler.log('ProfilePage.subscribeToUserFailed', error, {
            context: 'ProfilePage.followMutation',
            targetPubkey: target.pubkey,
          });
        }
      }
    },
    onSuccess: (_, target) => {
      if (currentUser) {
        queryClient.setQueryData<Profile[] | undefined>(
          ['profile', target.npub, 'followers'],
          (prev = []) => {
            if (prev.some((item) => item.npub === currentUser.npub)) {
              return prev;
            }
            return [...prev, { ...currentUser }];
          },
        );
        queryClient.setQueryData<Profile[] | undefined>(
          ['social', 'following', currentUser.npub],
          (prev = []) => {
            if (prev.some((item) => item.npub === target.npub)) {
              return prev;
            }
            return [...prev, { ...target }];
          },
        );
      }
      toast.success(`${target.displayName || target.npub} をフォローしました`);
    },
    onError: (error, target) => {
      if (error instanceof Error && error.message === 'ログインが必要です') {
        toast.error('フォローするにはログインが必要です');
        return;
      }
      errorHandler.log('ProfilePage.followFailed', error, {
        context: 'ProfilePage.followMutation',
        targetNpub: target.npub,
      });
      toast.error('フォローに失敗しました');
    },
  });

  const unfollowMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target) => {
      if (!currentUser) {
        throw new Error('ログインが必要です');
      }
      await TauriApi.unfollowUser(currentUser.npub, target.npub);
    },
    onSuccess: (_, target) => {
      if (currentUser) {
        queryClient.setQueryData<Profile[] | undefined>(
          ['profile', target.npub, 'followers'],
          (prev = []) => prev.filter((item) => item.npub !== currentUser.npub),
        );
        queryClient.setQueryData<Profile[] | undefined>(
          ['social', 'following', currentUser.npub],
          (prev = []) => prev.filter((item) => item.npub !== target.npub),
        );
      }
      toast.success(`${target.displayName || target.npub} のフォローを解除しました`);
    },
    onError: (error, target) => {
      if (error instanceof Error && error.message === 'ログインが必要です') {
        toast.error('フォロー解除にはログインが必要です');
        return;
      }
      errorHandler.log('ProfilePage.unfollowFailed', error, {
        context: 'ProfilePage.unfollowMutation',
        targetNpub: target.npub,
      });
      toast.error('フォロー解除に失敗しました');
    },
  });

  const handleFollowToggle = useCallback(() => {
    if (!profile || !canFollow) {
      return;
    }
    if (followMutation.isPending || unfollowMutation.isPending) {
      return;
    }
    if (isFollowing) {
      unfollowMutation.mutate(profile);
    } else {
      followMutation.mutate(profile);
    }
  }, [profile, canFollow, isFollowing, followMutation, unfollowMutation]);

  const followerCount = followersQuery.isLoading ? null : followers.length;
  const followingCount = followingQuery.isLoading ? null : following.length;
  const followButtonLabel = isCurrentUser
    ? 'あなた'
    : !canFollow
    ? 'ログインが必要'
    : isFollowing
    ? 'フォロー中'
    : 'フォロー';
  const isFollowProcessing =
    (followMutation.isPending && followMutation.variables?.npub === profile?.npub) ||
    (unfollowMutation.isPending && unfollowMutation.variables?.npub === profile?.npub);

  const handleCopyNpub = async (npub: string) => {
    try {
      await navigator.clipboard.writeText(npub);
      toast.success('npub をコピーしました');
    } catch (error) {
      toast.error('コピーに失敗しました');
    }
  };

  if (profileQuery.isLoading) {
    return (
      <div className="flex min-h-[50vh] items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!profile) {
    return (
      <div className="max-w-3xl mx-auto py-8 space-y-6">
        <Link
          to="/search"
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft className="h-4 w-4" />
          ユーザー検索に戻る
        </Link>

        <Card>
          <CardHeader>
            <CardTitle>ユーザーが見つかりませんでした</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground">
            <p className="text-sm leading-relaxed">
              指定されたユーザー（{userId}）のプロフィール情報が取得できませんでした。Nostr ネットワークの同期状況をご確認ください。
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const avatarSrc = resolveUserAvatarSrc(profile);
  const initials = getInitials(profile.displayName || profile.name || 'U');

  return (
    <div className="max-w-4xl mx-auto py-8 space-y-6">
      <Link
        to="/search"
        className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
      >
        <ArrowLeft className="h-4 w-4" />
        ユーザー検索に戻る
      </Link>

      <Card>
        <CardContent className="flex flex-col gap-6 p-6 md:flex-row md:items-center">
          <Avatar className="h-24 w-24">
            <AvatarImage src={avatarSrc} />
            <AvatarFallback>{initials}</AvatarFallback>
          </Avatar>
          <div className="flex-1 space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-2xl font-bold">{profile.displayName || 'ユーザー'}</h1>
              {profile.nip05 && <Badge variant="secondary">{profile.nip05}</Badge>}
            </div>
            {profile.name && <p className="text-sm text-muted-foreground">@{profile.name}</p>}
            <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              <code className="font-mono text-sm text-foreground break-all">{profile.npub}</code>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleCopyNpub(profile.npub)}
                aria-label="npubをコピー"
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
            <div className="flex flex-wrap gap-6 text-sm text-muted-foreground">
              <span>
                <span className="font-semibold text-foreground">
                  {followerCount ?? '…'}
                </span>{' '}
                フォロワー
              </span>
              <span>
                <span className="font-semibold text-foreground">
                  {followingCount ?? '…'}
                </span>{' '}
                フォロー中
              </span>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant={isFollowing ? 'secondary' : 'default'}
                disabled={!canFollow || isFollowProcessing}
                onClick={handleFollowToggle}
                className="min-w-[120px]"
              >
                {isFollowProcessing ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : isFollowing ? (
                  <UserCheck className="h-4 w-4 mr-2" />
                ) : (
                  <UserPlus className="h-4 w-4 mr-2" />
                )}
                {followButtonLabel}
              </Button>
              <Button variant="outline" size="sm" disabled className="min-w-[140px]">
                <MessageCircle className="h-4 w-4 mr-2" />
                メッセージ（準備中）
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>フォロー関係</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-6 md:grid-cols-2">
          <UserList
            title="フォロワー"
            users={followers}
            isLoading={followersQuery.isLoading}
            emptyText="フォロワーはいません。"
          />
          <UserList
            title="フォロー中"
            users={following}
            isLoading={followingQuery.isLoading}
            emptyText="フォロー中のユーザーはいません。"
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>自己紹介</CardTitle>
        </CardHeader>
        <CardContent>
          {profile.about ? (
            <p className="whitespace-pre-wrap leading-relaxed">{profile.about}</p>
          ) : (
            <p className="text-sm text-muted-foreground">自己紹介はまだありません。</p>
          )}
        </CardContent>
      </Card>

      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">投稿</h2>
          <span className="text-sm text-muted-foreground">{posts.length}件</span>
        </div>
        {postsQuery.isLoading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : posts.length === 0 ? (
          <Card>
            <CardContent className="py-12 text-center text-sm text-muted-foreground">
              まだ投稿がありません。
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-4">
            {posts.map((post) => (
              <PostCard key={post.id} post={post} data-testid="user-post-card" />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

interface UserListProps {
  title: string;
  users: Profile[];
  isLoading: boolean;
  emptyText: string;
}

function UserList({ title, users, isLoading, emptyText }: UserListProps) {
  return (
    <div>
      <h3 className="text-sm font-semibold text-foreground">{title}</h3>
      {isLoading ? (
        <div className="flex items-center gap-2 mt-3 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          読み込み中…
        </div>
      ) : users.length === 0 ? (
        <p className="mt-3 text-sm text-muted-foreground">{emptyText}</p>
      ) : (
        <div className="mt-3 space-y-3">
          {users.map((user) => {
            const avatarSrc = resolveUserAvatarSrc(user);
            const initials = getInitials(user.displayName || user.name || 'U');
            return (
              <div key={user.npub} className="flex items-center gap-3">
                <Avatar className="h-8 w-8">
                  <AvatarImage src={avatarSrc} />
                  <AvatarFallback>{initials}</AvatarFallback>
                </Avatar>
                <div className="min-w-0">
                  <Link
                    to="/profile/$userId"
                    params={{ userId: user.npub || user.id }}
                    className="text-sm font-medium hover:underline"
                  >
                    {user.displayName || user.name || 'ユーザー'}
                  </Link>
                  <p className="text-xs text-muted-foreground break-all">{user.npub}</p>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
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
