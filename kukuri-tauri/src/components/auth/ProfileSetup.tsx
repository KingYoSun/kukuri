import { useTranslation } from 'react-i18next';
import { useMemo, useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useAuthStore } from '@/stores/authStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { initializeNostr, updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { ProfileForm, type ProfileFormSubmitPayload, type ProfileFormValues } from './ProfileForm';
import { TauriApi } from '@/lib/api/tauri';
import { buildAvatarDataUrl, buildUserAvatarMetadata } from '@/lib/profile/avatar';
import { useProfileAvatarSync } from '@/hooks/useProfileAvatarSync';
import { useTheme } from '@/hooks/useTheme';
import { buildProfileSavePayload, collectUniqueSaveErrors } from '@/lib/profile/profileSave';
import { syncProfileQueryCaches } from '@/lib/profile/profileQuerySync';

export const PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS = 5_000;
export const PROFILE_SETUP_LOCAL_STEP_TIMEOUT_MS = 8_000;

const withTimeout = async <T,>(
  promise: Promise<T>,
  timeoutMs: number,
  label: string,
): Promise<T> => {
  let timeoutId: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timeoutId = setTimeout(() => {
          reject(new Error(`${label} timed out after ${timeoutMs}ms`));
        }, timeoutMs);
      }),
    ]);
  } finally {
    if (timeoutId !== undefined) {
      clearTimeout(timeoutId);
    }
  }
};

export function ProfileSetup() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { currentUser, updateUser } = useAuthStore();
  useTheme(); // Apply theme to HTML element
  const { publicProfile, showOnlineStatus } = usePrivacySettingsStore();
  const [isLoading, setIsLoading] = useState(false);
  const shouldNavigateRef = useRef(false);
  const { syncNow: syncAvatar } = useProfileAvatarSync({ autoStart: false });

  const initialProfile: ProfileFormValues = useMemo(
    () => ({
      name: currentUser?.name || '',
      displayName: currentUser?.displayName || '',
      about: currentUser?.about || '',
      picture: currentUser?.picture || '',
      nip05: currentUser?.nip05 || '',
    }),
    [
      currentUser?.about,
      currentUser?.displayName,
      currentUser?.name,
      currentUser?.nip05,
      currentUser?.picture,
    ],
  );

  const navigateHome = async () => {
    try {
      await navigate({ to: '/' });
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

  const handleSubmit = async (profile: ProfileFormSubmitPayload) => {
    if (!profile.name.trim()) {
      toast.error(t('auth.enterName'));
      return;
    }

    setIsLoading(true);
    shouldNavigateRef.current = false;
    const saveErrors: string[] = [];
    const pushSaveError = (stepLabel: string, error?: unknown) => {
      const details =
        error instanceof Error && error.message.trim().length > 0
          ? `${stepLabel}: ${error.message}`
          : stepLabel;
      saveErrors.push(details);
    };
    let updatedPicture = profile.picture || currentUser?.picture || '';
    let updatedAvatar = currentUser?.avatar ?? null;
    let nostrPicture =
      profile.picture || currentUser?.avatar?.nostrUri || currentUser?.picture || '';
    const accountNpub = currentUser?.npub?.trim() || '';

    try {
      try {
        await withTimeout(
          initializeNostr(),
          PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS,
          'initializeNostr',
        );
      } catch (nostrInitError) {
        pushSaveError(t('auth.profileSaveStepNostrInitialize'), nostrInitError);
        errorHandler.log('Failed to initialize Nostr for profile setup', nostrInitError, {
          context: 'ProfileSetup.handleSubmit.initializeNostr',
        });
      }

      if (currentUser?.npub) {
        try {
          await withTimeout(
            TauriApi.updatePrivacySettings({
              npub: currentUser.npub,
              publicProfile,
              showOnlineStatus,
            }),
            PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS,
            'updatePrivacySettings',
          );
        } catch (privacyError) {
          pushSaveError(t('auth.profileSaveStepPrivacy'), privacyError);
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

        try {
          const uploadResult = await withTimeout(
            TauriApi.uploadProfileAvatar({
              npub: currentUser.npub,
              data: profile.avatarFile.bytes,
              format: profile.avatarFile.format,
              accessLevel: 'contacts_only',
            }),
            PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS,
            'uploadProfileAvatar',
          );
          const fetched = await withTimeout(
            TauriApi.fetchProfileAvatar(currentUser.npub),
            PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS,
            'fetchProfileAvatar',
          );
          updatedPicture = buildAvatarDataUrl(fetched.format, fetched.data_base64);
          updatedAvatar = buildUserAvatarMetadata(currentUser.npub, uploadResult);
          nostrPicture = updatedAvatar.nostrUri;
        } catch (avatarError) {
          pushSaveError(t('auth.profileSaveStepAvatar'), avatarError);
          errorHandler.log('ProfileSetup.avatarUploadFailed', avatarError, {
            context: 'ProfileSetup.handleSubmit.avatarUpload',
          });
        }
      }

      if (!accountNpub) {
        throw new Error('Missing npub for profile setup');
      }
      if (!currentUser) {
        throw new Error('Missing current user for profile setup');
      }

      const payload = buildProfileSavePayload({
        npub: accountNpub,
        name: profile.name,
        displayName: profile.displayName,
        about: profile.about,
        picture: nostrPicture,
        nip05: profile.nip05,
        publicProfile,
        showOnlineStatus,
      });

      try {
        await withTimeout(
          TauriApi.updateUserProfile(payload.localProfile),
          PROFILE_SETUP_LOCAL_STEP_TIMEOUT_MS,
          'updateUserProfile',
        );
      } catch (localProfileError) {
        pushSaveError(t('auth.profileSaveStepLocalProfile'), localProfileError);
        errorHandler.log('ProfileSetup.localProfileUpdateFailed', localProfileError, {
          context: 'ProfileSetup.handleSubmit.updateUserProfile',
        });
      }

      try {
        await withTimeout(
          updateNostrMetadata(payload.nostrMetadata),
          PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS,
          'updateNostrMetadata',
        );
      } catch (nostrError) {
        pushSaveError(t('auth.profileSaveStepNostrMetadata'), nostrError);
        errorHandler.log('Failed to update Nostr metadata', nostrError, {
          context: 'ProfileSetup.handleSubmit.updateNostrMetadata',
        });
      }

      const updatedUser = {
        ...currentUser,
        name: payload.localProfile.name,
        displayName: payload.displayName,
        about: payload.localProfile.about,
        picture: updatedPicture,
        nip05: payload.localProfile.nip05,
        avatar: updatedAvatar,
        publicProfile,
        showOnlineStatus,
      };

      updateUser({
        name: updatedUser.name,
        displayName: updatedUser.displayName,
        about: updatedUser.about,
        picture: updatedUser.picture,
        nip05: updatedUser.nip05,
        avatar: updatedUser.avatar,
        publicProfile: updatedUser.publicProfile,
        showOnlineStatus: updatedUser.showOnlineStatus,
      });
      syncProfileQueryCaches(queryClient, updatedUser);

      try {
        await withTimeout(
          syncAvatar({ force: true }),
          PROFILE_SETUP_REMOTE_STEP_TIMEOUT_MS,
          'syncAvatar',
        );
      } catch (syncError) {
        pushSaveError(t('auth.profileSaveStepAvatarSync'), syncError);
        errorHandler.log('ProfileSetup.avatarSyncFailed', syncError, {
          context: 'ProfileSetup.handleSubmit',
        });
      } finally {
        useOfflineStore.getState().updateLastSyncedAt();
      }

      const uniqueErrors = collectUniqueSaveErrors(saveErrors);
      if (uniqueErrors.length > 0) {
        toast.error(
          t('auth.profileUpdateFailedWithDetails', {
            details: uniqueErrors.join(' / '),
          }),
        );
      } else {
        toast.success(t('auth.profileSetupSuccess'));
      }
      shouldNavigateRef.current = true;
    } catch (error) {
      toast.error(t('auth.profileSetupFailed'));
      errorHandler.log('Profile setup failed', error, {
        context: 'ProfileSetup.handleSubmit',
      });
    } finally {
      setIsLoading(false);
      if (shouldNavigateRef.current) {
        await navigateHome();
      }
    }
  };

  const handleSkip = () => {
    void navigateHome();
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-background">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle>{t('auth.profileSetup')}</CardTitle>
          <CardDescription>{t('auth.profileSetupDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <ProfileForm
            initialValues={initialProfile}
            onSubmit={handleSubmit}
            onSkip={handleSkip}
            skipLabel={t('auth.setupLater')}
            submitLabel={isLoading ? t('auth.saving') : t('auth.completeSetup')}
            isSubmitting={isLoading}
          />
        </CardContent>
      </Card>
    </div>
  );
}
