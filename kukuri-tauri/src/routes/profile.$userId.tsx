import { createFileRoute, Link } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import {
  Avatar,
  AvatarFallback,
  AvatarImage,
} from '@/components/ui/avatar';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Loader2, Copy, ArrowLeft } from 'lucide-react';
import { TauriApi } from '@/lib/api/tauri';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import { resolveUserAvatarSrc } from '@/lib/profile/avatarDisplay';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import type { Post } from '@/stores';
import { PostCard } from '@/components/posts/PostCard';
import { toast } from 'sonner';

export const Route = createFileRoute('/profile/$userId')({
  component: ProfilePage,
});

function ProfilePage() {
  const { userId } = Route.useParams();

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
    enabled: !!profile,
    queryFn: async () => {
      if (!profile) return [] as Post[];
      const apiPosts = await TauriApi.getPosts({
        author_pubkey: profile.pubkey,
        pagination: { limit: 50 },
      });
      return Promise.all(apiPosts.map((post) => mapPostResponseToDomain(post)));
    },
  });

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
  const initials = (profile.displayName || profile.name || 'U')
    .split(' ')
    .map((n) => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);

  const posts = postsQuery.data ?? [];

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
            {profile.name && (
              <p className="text-sm text-muted-foreground">@{profile.name}</p>
            )}
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
            <div className="flex flex-wrap gap-2">
              <Button size="sm" disabled>
                フォロー（準備中）
              </Button>
              <Button variant="outline" size="sm" disabled>
                メッセージ（準備中）
              </Button>
            </div>
          </div>
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
