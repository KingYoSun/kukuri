import { useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { useAuthStore } from '@/stores/authStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { Upload, User } from 'lucide-react';

export function ProfileSetup() {
  const navigate = useNavigate();
  const { currentUser, updateUser } = useAuthStore();
  const [isLoading, setIsLoading] = useState(false);
  
  const [profile, setProfile] = useState({
    name: currentUser?.name || '',
    displayName: currentUser?.displayName || '',
    about: currentUser?.about || '',
    picture: currentUser?.picture || '',
    nip05: currentUser?.nip05 || '',
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!profile.name.trim()) {
      toast.error('名前を入力してください');
      return;
    }

    setIsLoading(true);
    try {
      // Nostrプロフィールメタデータを更新
      await updateNostrMetadata({
        name: profile.name,
        display_name: profile.displayName || profile.name,
        about: profile.about,
        picture: profile.picture,
        nip05: profile.nip05,
      });

      // ローカルストアを更新
      updateUser({
        name: profile.name,
        displayName: profile.displayName || profile.name,
        about: profile.about,
        picture: profile.picture,
        nip05: profile.nip05,
      });

      toast.success('プロフィールを設定しました');
      await navigate({ to: '/' });
    } catch (error) {
      toast.error('プロフィールの設定に失敗しました');
      console.error('Profile setup failed:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSkip = () => {
    navigate({ to: '/' });
  };

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map(word => word[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle>プロフィール設定</CardTitle>
          <CardDescription>
            あなたの情報を設定しましょう
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* アバター */}
            <div className="flex flex-col items-center space-y-2">
              <Avatar className="w-24 h-24">
                <AvatarImage src={profile.picture} />
                <AvatarFallback>
                  {profile.name ? getInitials(profile.name) : <User className="w-12 h-12" />}
                </AvatarFallback>
              </Avatar>
              <Button type="button" variant="outline" size="sm">
                <Upload className="w-4 h-4 mr-2" />
                画像をアップロード
              </Button>
              <p className="text-xs text-muted-foreground">
                またはURLを下に入力
              </p>
            </div>

            {/* 基本情報 */}
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="name">名前 *</Label>
                <Input
                  id="name"
                  value={profile.name}
                  onChange={(e) => setProfile({ ...profile, name: e.target.value })}
                  placeholder="表示名"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="displayName">表示名</Label>
                <Input
                  id="displayName"
                  value={profile.displayName}
                  onChange={(e) => setProfile({ ...profile, displayName: e.target.value })}
                  placeholder="@handle（省略可）"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="about">自己紹介</Label>
                <Textarea
                  id="about"
                  value={profile.about}
                  onChange={(e) => setProfile({ ...profile, about: e.target.value })}
                  placeholder="あなたについて教えてください"
                  rows={3}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="picture">アバター画像URL</Label>
                <Input
                  id="picture"
                  type="url"
                  value={profile.picture}
                  onChange={(e) => setProfile({ ...profile, picture: e.target.value })}
                  placeholder="https://example.com/avatar.jpg"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="nip05">NIP-05認証</Label>
                <Input
                  id="nip05"
                  value={profile.nip05}
                  onChange={(e) => setProfile({ ...profile, nip05: e.target.value })}
                  placeholder="user@example.com"
                />
                <p className="text-xs text-muted-foreground">
                  Nostr認証用のメールアドレス形式の識別子（省略可）
                </p>
              </div>
            </div>

            {/* ボタン */}
            <div className="flex gap-3">
              <Button
                type="button"
                variant="outline"
                onClick={handleSkip}
                className="flex-1"
              >
                後で設定
              </Button>
              <Button type="submit" className="flex-1" disabled={isLoading}>
                {isLoading ? '保存中...' : '設定を完了'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}