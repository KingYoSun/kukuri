import { useMemo, useCallback, useEffect, useRef, useState } from 'react';
import { createFileRoute, Link } from '@tanstack/react-router';
import {
  useQuery,
  useMutation,
  useQueryClient,
  useInfiniteQuery,
  type InfiniteData,
} from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Loader2, Copy, ArrowLeft, UserPlus, UserCheck, MessageCircle } from 'lucide-react';
import { TauriApi, type FollowListSort } from '@/lib/api/tauri';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import type { Post, Profile } from '@/stores';
import { PostCard } from '@/components/posts/PostCard';
import { toast } from 'sonner';
import { useAuthStore } from '@/stores';
import { useDirectMessageStore } from '@/stores/directMessageStore';
import { errorHandler } from '@/lib/errorHandler';
import { subscribeToUser } from '@/lib/api/nostr';
import { useDebounce } from '@/hooks/useDebounce';

export const Route = createFileRoute('/profile/$userId')({
  component: ProfilePage,
});

type ProfileListPage = {
  items: Profile[];
  nextCursor: string | null;
  hasMore: boolean;
  totalCount: number;
};

const getFollowSortOptions = (t: (key: string) => string): Array<{ value: FollowListSort; label: string }> => [
  { value: 'recent', label: t('profile.sortRecent') },
  { value: 'oldest', label: t('profile.sortOldest') },
  { value: 'name_asc', label: t('profile.sortNameAsc') },
  { value: 'name_desc', label: t('profile.sortNameDesc') },
];

const FOLLOW_PAGE_SIZE = 25;

function matchesProfileSearch(profile: Profile, search?: string) {
  if (!search) {
    return true;
  }
  const normalized = search.trim().toLowerCase();
  if (!normalized) {
    return true;
  }
  const display = (profile.displayName ?? '').toLowerCase();
  const name = (profile.name ?? '').toLowerCase();
  const npub = profile.npub.toLowerCase();
  return display.includes(normalized) || name.includes(normalized) || npub.includes(normalized);
}

function ProfilePage() {
  const { t } = useTranslation();
  const { userId } = Route.useParams();
  const queryClient = useQueryClient();
  const currentUser = useAuthStore((state) => state.currentUser);
  const viewerNpub = currentUser?.npub ?? null;

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

  const openDirectMessage = useDirectMessageStore((state) => state.openDialog);
  const [followersSort, setFollowersSort] = useState<FollowListSort>('recent');
  const [followersSearchInput, setFollowersSearchInput] = useState('');
  const debouncedFollowersSearch = useDebounce(followersSearchInput, 300);
  const followerSearchTerm = debouncedFollowersSearch.trim();
  const followerSearchQuery = followerSearchTerm.length > 0 ? followerSearchTerm : undefined;

  const [followingSort, setFollowingSort] = useState<FollowListSort>('recent');
  const [followingSearchInput, setFollowingSearchInput] = useState('');
  const debouncedFollowingSearch = useDebounce(followingSearchInput, 300);
  const followingSearchTerm = debouncedFollowingSearch.trim();
  const followingSearchQuery = followingSearchTerm.length > 0 ? followingSearchTerm : undefined;

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

  const followersQuery = useInfiniteQuery<
    ProfileListPage,
    Error,
    InfiniteData<ProfileListPage>,
    ['profile', string, 'followers', FollowListSort, string, string | null],
    string | null
  >({
    queryKey: [
      'profile',
      profile?.npub ?? userId,
      'followers',
      followersSort,
      followerSearchQuery ?? '',
      viewerNpub,
    ],
    enabled: Boolean(profile),
    retry: false,
    initialPageParam: null,
    queryFn: async ({ pageParam }) => {
      if (!profile) {
        return { items: [], nextCursor: null, hasMore: false, totalCount: 0 };
      }
      const response = await TauriApi.getFollowers({
        npub: profile.npub,
        cursor: pageParam,
        limit: FOLLOW_PAGE_SIZE,
        sort: followersSort,
        search: followerSearchQuery,
        viewerNpub,
      });
      return {
        items: response.items.map(mapUserProfileToUser),
        nextCursor: response.nextCursor,
        hasMore: response.hasMore,
        totalCount: response.totalCount,
      };
    },
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? (lastPage.nextCursor ?? undefined) : undefined,
  });

  const followingQuery = useInfiniteQuery<
    ProfileListPage,
    Error,
    InfiniteData<ProfileListPage>,
    ['profile', string, 'following', FollowListSort, string, string | null],
    string | null
  >({
    queryKey: [
      'profile',
      profile?.npub ?? userId,
      'following',
      followingSort,
      followingSearchQuery ?? '',
      viewerNpub,
    ],
    enabled: Boolean(profile),
    retry: false,
    initialPageParam: null,
    queryFn: async ({ pageParam }) => {
      if (!profile) {
        return { items: [], nextCursor: null, hasMore: false, totalCount: 0 };
      }
      const response = await TauriApi.getFollowing({
        npub: profile.npub,
        cursor: pageParam,
        limit: FOLLOW_PAGE_SIZE,
        sort: followingSort,
        search: followingSearchQuery,
        viewerNpub,
      });
      return {
        items: response.items.map(mapUserProfileToUser),
        nextCursor: response.nextCursor,
        hasMore: response.hasMore,
        totalCount: response.totalCount,
      };
    },
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? (lastPage.nextCursor ?? undefined) : undefined,
  });

  const {
    data: followersData,
    isLoading: followersLoading,
    isFetchingNextPage: followersFetchingNext,
    hasNextPage: followersHasNext,
    fetchNextPage: fetchFollowersNext,
  } = followersQuery;

  const {
    data: followingData,
    isLoading: followingLoading,
    isFetchingNextPage: followingFetchingNext,
    hasNextPage: followingHasNext,
    fetchNextPage: fetchFollowingNext,
  } = followingQuery;

  useEffect(() => {
    if (followersQuery.isError && followersQuery.error) {
      errorHandler.log('ProfilePage.followersFetchFailed', followersQuery.error, {
        context: 'ProfilePage.followersQuery',
        metadata: { userId: profile?.npub ?? userId },
      });
      toast.error(t('profile.fetchFollowersFailed'));
    }
  }, [followersQuery.isError, followersQuery.error, profile?.npub, userId, t]);

  useEffect(() => {
    if (followingQuery.isError && followingQuery.error) {
      errorHandler.log('ProfilePage.followingFetchFailed', followingQuery.error, {
        context: 'ProfilePage.followingQuery',
        metadata: { userId: profile?.npub ?? userId },
      });
      toast.error(t('profile.fetchFollowingFailed'));
    }
  }, [followingQuery.isError, followingQuery.error, profile?.npub, userId, t]);

  const followers = followersData?.pages.flatMap((page) => page.items) ?? [];
  const following = followingData?.pages.flatMap((page) => page.items) ?? [];
  const followersTotalCount = followersData?.pages[0]?.totalCount ?? 0;
  const followingTotalCount = followingData?.pages[0]?.totalCount ?? 0;

  const handleFollowersLoadMore = useCallback(() => {
    if (followersHasNext && !followersFetchingNext) {
      void fetchFollowersNext();
    }
  }, [followersHasNext, followersFetchingNext, fetchFollowersNext]);

  const handleFollowingLoadMore = useCallback(() => {
    if (followingHasNext && !followingFetchingNext) {
      void fetchFollowingNext();
    }
  }, [followingHasNext, followingFetchingNext, fetchFollowingNext]);

  const posts = postsQuery.data ?? [];

  const isCurrentUser = Boolean(profile && currentUser?.npub === profile.npub);
  const canFollow = Boolean(currentUser) && Boolean(profile) && !isCurrentUser;
  const canMessage = Boolean(currentUser) && Boolean(profile) && !isCurrentUser;

  const isFollowing = useMemo(() => {
    if (!profile || !currentUser || isCurrentUser) {
      return false;
    }
    return followers.some((follower) => follower.npub === currentUser.npub);
  }, [followers, currentUser, profile, isCurrentUser]);

  const followMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target) => {
      if (!currentUser) {
        throw new Error(t('profile.loginRequired'));
      }
      await TauriApi.followUser(currentUser.npub, target.npub);
      if (target.pubkey) {
        try {
          await subscribeToUser(target.pubkey);
        } catch (error) {
          errorHandler.log('ProfilePage.subscribeToUserFailed', error, {
            context: 'ProfilePage.followMutation',
            metadata: { targetPubkey: target.pubkey },
          });
        }
      }
    },
    onSuccess: (_, target) => {
      if (currentUser) {
        const followerProfile = { ...currentUser };
        const followersKey = [
          'profile',
          target.npub,
          'followers',
          followersSort,
          followerSearchQuery ?? '',
          viewerNpub,
        ] as const;
        const matchesFilter = matchesProfileSearch(followerProfile, followerSearchQuery);
        queryClient.setQueryData<InfiniteData<ProfileListPage> | undefined>(
          followersKey,
          (prev) => {
            if (!matchesFilter) {
              return prev;
            }
            if (!prev) {
              return {
                pages: [
                  {
                    items: [followerProfile],
                    nextCursor: null,
                    hasMore: false,
                    totalCount: 1,
                  },
                ],
                pageParams: [undefined],
              };
            }
            const exists = prev.pages.some((page) =>
              page.items.some((item) => item.npub === followerProfile.npub),
            );
            if (exists) {
              return prev;
            }
            const pages = prev.pages.map((page, index) => {
              if (index === 0) {
                const items = [followerProfile, ...page.items];
                const trimmedItems = items.slice(0, FOLLOW_PAGE_SIZE);
                return {
                  ...page,
                  items: trimmedItems,
                  totalCount: page.totalCount + 1,
                  hasMore: page.hasMore || items.length > FOLLOW_PAGE_SIZE,
                };
              }
              return {
                ...page,
                totalCount: page.totalCount + 1,
              };
            });
            return {
              ...prev,
              pages,
            };
          },
        );
        void queryClient.invalidateQueries({
          queryKey: ['profile', target.npub, 'followers'],
          exact: false,
        });
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
      toast.success(t('profile.followSuccess', { name: target.displayName || target.npub }));
    },
    onError: (error, target) => {
      if (error instanceof Error && error.message === t('profile.loginRequired')) {
        toast.error(t('profile.followLoginRequired'));
        return;
      }
      errorHandler.log('ProfilePage.followFailed', error, {
        context: 'ProfilePage.followMutation',
        metadata: { targetNpub: target.npub },
      });
      toast.error(t('profile.followFailed'));
    },
  });

  const unfollowMutation = useMutation<void, unknown, Profile>({
    mutationFn: async (target) => {
      if (!currentUser) {
        throw new Error(t('profile.loginRequired'));
      }
      await TauriApi.unfollowUser(currentUser.npub, target.npub);
    },
    onSuccess: (_, target) => {
      if (currentUser) {
        const followersKey = [
          'profile',
          target.npub,
          'followers',
          followersSort,
          followerSearchQuery ?? '',
          viewerNpub,
        ] as const;
        queryClient.setQueryData<InfiniteData<ProfileListPage> | undefined>(
          followersKey,
          (prev) => {
            if (!prev) {
              return prev;
            }
            const updatedPages = prev.pages.map((page) => ({
              ...page,
              items: page.items.filter((item) => item.npub !== currentUser.npub),
            }));
            const removedCount = prev.pages.reduce((acc, page, index) => {
              const removed = page.items.length - updatedPages[index].items.length;
              return acc + removed;
            }, 0);
            if (removedCount === 0) {
              return prev;
            }
            const pages = updatedPages.map((page) => ({
              ...page,
              totalCount: Math.max(page.totalCount - removedCount, 0),
            }));
            return {
              ...prev,
              pages,
            };
          },
        );
        void queryClient.invalidateQueries({
          queryKey: ['profile', target.npub, 'followers'],
          exact: false,
        });
        queryClient.setQueryData<Profile[] | undefined>(
          ['social', 'following', currentUser.npub],
          (prev = []) => prev.filter((item) => item.npub !== target.npub),
        );
      }
      toast.success(t('profile.unfollowSuccess', { name: target.displayName || target.npub }));
    },
    onError: (error, target) => {
      if (error instanceof Error && error.message === t('profile.loginRequired')) {
        toast.error(t('profile.unfollowLoginRequired'));
        return;
      }
      errorHandler.log('ProfilePage.unfollowFailed', error, {
        context: 'ProfilePage.unfollowMutation',
        metadata: { targetNpub: target.npub },
      });
      toast.error(t('profile.unfollowFailed'));
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

  const followerCount = followersLoading ? null : followersTotalCount;
  const followingCount = followingLoading ? null : followingTotalCount;
  const followButtonLabel = isCurrentUser
    ? t('profile.you')
    : !canFollow
      ? t('profile.loginRequired')
      : isFollowing
        ? t('profile.followingStatus')
        : t('profile.follow');
  const isFollowProcessing =
    (followMutation.isPending && followMutation.variables?.npub === profile?.npub) ||
    (unfollowMutation.isPending && unfollowMutation.variables?.npub === profile?.npub);

  const handleOpenDirectMessage = useCallback(() => {
    if (!profile) {
      return;
    }
    if (!currentUser) {
      toast.error(t('profile.messageLoginRequired'));
      return;
    }
    openDirectMessage(profile.npub);
  }, [currentUser, openDirectMessage, profile, t]);

  const handleCopyNpub = async (npub: string) => {
    try {
      await navigator.clipboard.writeText(npub);
      toast.success(t('profile.copyNpubSuccess'));
    } catch {
      toast.error(t('profile.copyFailed'));
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
          {t('profile.backToSearch')}
        </Link>

        <Card>
          <CardHeader>
            <CardTitle>{t('profile.userNotFound')}</CardTitle>
          </CardHeader>
          <CardContent className="text-muted-foreground">
            <p className="text-sm leading-relaxed">
              {t('profile.userNotFoundDescription', { userId })}
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
        {t('profile.backToSearch')}
      </Link>

      <Card>
        <CardContent className="flex flex-col gap-6 p-6 md:flex-row md:items-center">
          <Avatar className="h-24 w-24">
            <AvatarImage src={avatarSrc} />
            <AvatarFallback>{initials}</AvatarFallback>
          </Avatar>
          <div className="flex-1 space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-2xl font-bold">{profile.displayName || t('profile.user')}</h1>
              {profile.nip05 && <Badge variant="secondary">{profile.nip05}</Badge>}
            </div>
            {profile.name && <p className="text-sm text-muted-foreground">@{profile.name}</p>}
            <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              <code className="font-mono text-sm text-foreground break-all">{profile.npub}</code>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => handleCopyNpub(profile.npub)}
                aria-label={t('profile.copyNpub')}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
            <div className="flex flex-wrap gap-6 text-sm text-muted-foreground">
              <span>
                <span className="font-semibold text-foreground">{followerCount ?? '…'}</span>{' '}
                {t('profile.followers')}
              </span>
              <span>
                <span className="font-semibold text-foreground">{followingCount ?? '…'}</span>{' '}
                {t('profile.following')}
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
              <Button
                variant="outline"
                size="sm"
                disabled={!canMessage}
                className="min-w-[140px]"
                onClick={() => handleOpenDirectMessage()}
              >
                <MessageCircle className="h-4 w-4 mr-2" />
                {t('profile.message')}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('profile.followRelations')}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-6 md:grid-cols-2">
          <UserList
            title={t('profile.followers')}
            users={followers}
            isLoading={followersLoading}
            emptyText={t('profile.noFollowers')}
            onLoadMore={handleFollowersLoadMore}
            hasNextPage={Boolean(followersHasNext)}
            isFetchingNextPage={followersFetchingNext}
            sort={followersSort}
            sortOptions={getFollowSortOptions(t)}
            onSortChange={setFollowersSort}
            searchTerm={followersSearchInput}
            onSearchChange={setFollowersSearchInput}
            totalCount={followersTotalCount}
          />
          <UserList
            title={t('profile.following')}
            users={following}
            isLoading={followingLoading}
            emptyText={t('profile.noFollowing')}
            onLoadMore={handleFollowingLoadMore}
            hasNextPage={Boolean(followingHasNext)}
            isFetchingNextPage={followingFetchingNext}
            sort={followingSort}
            sortOptions={getFollowSortOptions(t)}
            onSortChange={setFollowingSort}
            searchTerm={followingSearchInput}
            onSearchChange={setFollowingSearchInput}
            totalCount={followingTotalCount}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('profile.bio')}</CardTitle>
        </CardHeader>
        <CardContent>
          {profile.about ? (
            <p className="whitespace-pre-wrap leading-relaxed">{profile.about}</p>
          ) : (
            <p className="text-sm text-muted-foreground">{t('profile.noBio')}</p>
          )}
        </CardContent>
      </Card>

      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">{t('profile.posts')}</h2>
          <span className="text-sm text-muted-foreground">{t('profile.postsCount', { count: posts.length })}</span>
        </div>
        {postsQuery.isLoading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : posts.length === 0 ? (
          <Card>
            <CardContent className="py-12 text-center text-sm text-muted-foreground">
              {t('profile.noPosts')}
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

export { ProfilePage };

interface UserListProps {
  title: string;
  users: Profile[];
  isLoading: boolean;
  emptyText: string;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  onLoadMore?: () => void;
  sort: FollowListSort;
  sortOptions: Array<{ value: FollowListSort; label: string }>;
  onSortChange: (value: FollowListSort) => void;
  searchTerm: string;
  onSearchChange: (value: string) => void;
  totalCount: number;
}

function UserList({
  title,
  users,
  isLoading,
  emptyText,
  hasNextPage = false,
  isFetchingNextPage = false,
  onLoadMore,
  sort,
  sortOptions,
  onSortChange,
  searchTerm,
  onSearchChange,
  totalCount,
}: UserListProps) {
  const { t } = useTranslation();
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!hasNextPage || !onLoadMore) {
      return;
    }
    const sentinel = sentinelRef.current;
    if (!sentinel) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting && !isFetchingNextPage) {
            onLoadMore();
          }
        });
      },
      { rootMargin: '200px 0px' },
    );

    observer.observe(sentinel);

    return () => {
      observer.disconnect();
    };
  }, [hasNextPage, isFetchingNextPage, onLoadMore]);

  return (
    <div>
      <div className="flex flex-col gap-1">
        <div className="flex items-baseline justify-between gap-2">
          <h3 className="text-sm font-semibold text-foreground">{title}</h3>
          <span className="text-xs text-muted-foreground">
            {t('profile.showing', { current: users.length, total: totalCount })}
          </span>
        </div>
        <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <Select value={sort} onValueChange={(value) => onSortChange(value as FollowListSort)}>
            <SelectTrigger className="md:w-40">
              <SelectValue placeholder={t('profile.sort')} />
            </SelectTrigger>
            <SelectContent>
              {sortOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            value={searchTerm}
            onChange={(event) => onSearchChange(event.target.value)}
            placeholder={t('profile.searchUsers')}
            className="md:w-48"
            aria-label={t('profile.searchUsersLabel', { title })}
          />
        </div>
      </div>
      {isLoading ? (
        <div className="flex items-center gap-2 mt-3 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          {t('profile.loading')}
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
                    {user.displayName || user.name || t('profile.user')}
                  </Link>
                  <p className="text-xs text-muted-foreground break-all">{user.npub}</p>
                </div>
              </div>
            );
          })}
          <div ref={sentinelRef} className="h-1 w-full" />
          {isFetchingNextPage && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              <span>{t('profile.loadingMore')}</span>
            </div>
          )}
          {!hasNextPage && users.length > 0 && (
            <div className="text-xs text-muted-foreground">{t('profile.allUsersShown')}</div>
          )}
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
