import { useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useAuthStore } from '@/stores/authStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { ProfileForm, type ProfileFormSubmitPayload, type ProfileFormValues } from './ProfileForm';
import { TauriApi } from '@/lib/api/tauri';
import { buildAvatarDataUrl, buildUserAvatarMetadata } from '@/lib/profile/avatar';
import { useProfileAvatarSync } from '@/hooks/useProfileAvatarSync';

export function ProfileSetup() {
  const navigate = useNavigate();
  const { currentUser, updateUser } = useAuthStore();
  const { publicProfile, showOnlineStatus } = usePrivacySettingsStore();
  const [isLoading, setIsLoading] = useState(false);
  const { syncNow: syncAvatar } = useProfileAvatarSync({ autoStart: false });

  const initialProfile: ProfileFormValues = {
    name: currentUser?.name || '',
    displayName: currentUser?.displayName || '',
    about: currentUser?.about || '',
    picture: currentUser?.picture || '',
    nip05: currentUser?.nip05 || '',
  };

  const handleSubmit = async (profile: ProfileFormSubmitPayload) => {
    if (!profile.name.trim()) {
      toast.error('名前を入力してください');
      return;
    }

    setIsLoading(true);
    try {
      let updatedPicture = profile.picture || currentUser?.picture || '';
      let updatedAvatar = currentUser?.avatar ?? null;
      let nostrPicture =
        profile.picture || currentUser?.avatar?.nostrUri || currentUser?.picture || '';

      if (profile.avatarFile) {
        if (!currentUser?.npub) {
          throw new Error('Missing npub for avatar upload');
        }

        const uploadResult = await TauriApi.uploadProfileAvatar({
          npub: currentUser.npub,
          data: profile.avatarFile.bytes,
          format: profile.avatarFile.format,
          accessLevel: 'contacts_only',
        });
        const fetched = await TauriApi.fetchProfileAvatar(currentUser.npub);
        updatedPicture = buildAvatarDataUrl(fetched.format, fetched.data_base64);
        updatedAvatar = buildUserAvatarMetadata(currentUser.npub, uploadResult);
        nostrPicture = updatedAvatar.nostrUri;
      }

      if (!currentUser?.npub) {
        throw new Error('Missing npub for profile setup');
      }

      await TauriApi.updatePrivacySettings({
        npub: currentUser.npub,
        publicProfile,
        showOnlineStatus,
      });

      // Nostrプロフィールメタデータを更新
      await updateNostrMetadata({
        name: profile.name,
        display_name: profile.displayName || profile.name,
        about: profile.about,
        picture: nostrPicture,
        nip05: profile.nip05,
        kukuri_privacy: {
          public_profile: publicProfile,
          show_online_status: showOnlineStatus,
        },
      });

      // ローカルストアを更新
      updateUser({
        name: profile.name,
        displayName: profile.displayName || profile.name,
        about: profile.about,
        picture: updatedPicture,
        nip05: profile.nip05,
        avatar: updatedAvatar,
        publicProfile,
        showOnlineStatus,
      });

      toast.success('プロフィールを設定しました');
      try {
        await syncAvatar({ force: true });
      } catch (syncError) {
        errorHandler.log('ProfileSetup.avatarSyncFailed', syncError, {
          context: 'ProfileSetup.handleSubmit',
        });
      }
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
