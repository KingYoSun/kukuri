import { useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useAuthStore } from '@/stores/authStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { initializeNostr, updateNostrMetadata } from '@/lib/api/nostr';
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
  const [showForm, setShowForm] = useState(true);
  const isE2EMode =
    import.meta.env.TAURI_ENV_DEBUG === 'true' || import.meta.env.VITE_ENABLE_E2E === 'true';
  const shouldNavigateRef = useRef(false);
  const { syncNow: syncAvatar } = useProfileAvatarSync({ autoStart: false });

  const initialProfile: ProfileFormValues = {
    name: currentUser?.name || '',
    displayName: currentUser?.displayName || '',
    about: currentUser?.about || '',
    picture: currentUser?.picture || '',
    nip05: currentUser?.nip05 || '',
  };

  const navigateHome = () => {
    try {
      navigate({ to: '/' });
    } catch (navError) {
      errorHandler.log('ProfileSetup.navigateFailed', navError, {
        context: 'ProfileSetup.navigateHome',
      });
      try {
        window.location.replace('/');
      } catch {
        // ルーターが動かない場合のフォールバック
      }
    }
  };

  const hideFormAndNavigate = () => {
    setShowForm(false);
    navigateHome();
  };

  const handleSubmit = async (profile: ProfileFormSubmitPayload) => {
    if (!profile.name.trim()) {
      toast.error('名前を入力してください');
      return;
    }

    setIsLoading(true);
    shouldNavigateRef.current = false;
    let updatedPicture = profile.picture || currentUser?.picture || '';
    let updatedAvatar = currentUser?.avatar ?? null;
    let nostrPicture =
      profile.picture || currentUser?.avatar?.nostrUri || currentUser?.picture || '';
    const displayName = profile.displayName || profile.name;
    const applyLocalUpdate = () => {
      updateUser({
        name: profile.name,
        displayName,
        about: profile.about,
        picture: updatedPicture,
        nip05: profile.nip05,
        avatar: updatedAvatar,
        publicProfile,
        showOnlineStatus,
      });
    };

    try {
      if (isE2EMode) {
        applyLocalUpdate();
        toast.success('プロフィールを設定しました');
        shouldNavigateRef.current = true;
        return;
      }

      try {
        await initializeNostr();
      } catch (nostrInitError) {
        errorHandler.log('Failed to initialize Nostr for profile setup', nostrInitError, {
          context: 'ProfileSetup.handleSubmit.initializeNostr',
        });
      }

      if (currentUser?.npub) {
        try {
          await TauriApi.updatePrivacySettings({
            npub: currentUser.npub,
            publicProfile,
            showOnlineStatus,
          });
        } catch (privacyError) {
          // プライバシー更新に失敗してもログだけ出して続行
          errorHandler.log('Privacy update skipped (proceeding anyway)', privacyError, {
            context: 'ProfileSetup.handleSubmit.updatePrivacySettings',
          });
        }
      }

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

      // Nostr???????????????E2E????????????????????
      if (!isE2EMode) {
        try {
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
        } catch (nostrError) {
          errorHandler.log('Failed to update Nostr metadata', nostrError, {
            context: 'ProfileSetup.handleSubmit.updateNostrMetadata',
          });
          throw nostrError;
        }
      }

      // 繝ｭ繝ｼ繧ｫ繝ｫ繧ｹ繝医い繧呈峩譁ｰ
      updateUser({
        name: profile.name,
        displayName,
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
      shouldNavigateRef.current = true;
    } catch (error) {
      toast.error('プロフィールの設定に失敗しました');
      errorHandler.log('Profile setup failed', error, {
        context: 'ProfileSetup.handleSubmit',
      });
      if (isE2EMode) {
        applyLocalUpdate();
        shouldNavigateRef.current = true;
      }
    } finally {
      setIsLoading(false);
      if (shouldNavigateRef.current || isE2EMode) {
        hideFormAndNavigate();
      }
    }
  };

  const handleSkip = () => {
    hideFormAndNavigate();
  };

  if (!showForm) {
    return null;
  }

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
            onSubmitFinally={() => {
              if (!shouldNavigateRef.current) {
                return;
              }
              // プロフィール送信後にホーム遷移（失敗しても処理は継続）
              try {
                navigate({ to: '/' });
              } catch (navError) {
                errorHandler.log('ProfileSetup.finallyNavigateFailed', navError, {
                  context: 'ProfileSetup.onSubmitFinally',
                });
              }
            }}
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
