import { useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useAuthStore } from '@/stores/authStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { ProfileForm, type ProfileFormValues } from './ProfileForm';

export function ProfileSetup() {
  const navigate = useNavigate();
  const { currentUser, updateUser } = useAuthStore();
  const [isLoading, setIsLoading] = useState(false);

  const initialProfile: ProfileFormValues = {
    name: currentUser?.name || '',
    displayName: currentUser?.displayName || '',
    about: currentUser?.about || '',
    picture: currentUser?.picture || '',
    nip05: currentUser?.nip05 || '',
  };

  const handleSubmit = async (profile: ProfileFormValues) => {
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
      errorHandler.log('Profile setup failed', error, {
        context: 'ProfileSetup.handleSubmit',
      });
    } finally {
      setIsLoading(false);
    }
  };

  const handleSkip = () => {
    navigate({ to: '/' });
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle>プロフィール設定</CardTitle>
          <CardDescription>あなたの情報を設定しましょう</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <ProfileForm
            initialValues={initialProfile}
            onSubmit={handleSubmit}
            onSkip={handleSkip}
            skipLabel="後で設定"
            submitLabel={isLoading ? '保存中...' : '設定を完了'}
            isSubmitting={isLoading}
          />
        </CardContent>
      </Card>
    </div>
  );
}
