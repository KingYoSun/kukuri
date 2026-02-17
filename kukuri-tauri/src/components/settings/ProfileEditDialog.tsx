import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  ProfileForm,
  type ProfileFormSubmitPayload,
  type ProfileFormValues,
} from '@/components/auth/ProfileForm';
import { useAuthStore } from '@/stores/authStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import { TauriApi } from '@/lib/api/tauri';
import { buildAvatarDataUrl, buildUserAvatarMetadata } from '@/lib/profile/avatar';
import { useProfileAvatarSync } from '@/hooks/useProfileAvatarSync';

interface ProfileEditDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ProfileEditDialog({ open, onOpenChange }: ProfileEditDialogProps) {
  const { t } = useTranslation();
  const { currentUser, updateUser } = useAuthStore();
  const { publicProfile, showOnlineStatus } = usePrivacySettingsStore();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const shouldCloseOnFinallyRef = useRef(false);
  const { syncNow: syncAvatar } = useProfileAvatarSync({ autoStart: false });

  const initialValues: ProfileFormValues = useMemo(
    () => ({
      name: currentUser?.name || '',
      displayName: currentUser?.displayName || '',
      about: currentUser?.about || '',
      picture: currentUser?.picture || '',
      nip05: currentUser?.nip05 || '',
    }),
    [currentUser],
  );

  const handleClose = () => {
    shouldCloseOnFinallyRef.current = false;
    onOpenChange(false);
  };

  useEffect(() => {
    if (!open) {
      shouldCloseOnFinallyRef.current = false;
    }
  }, [open]);

  const handleSubmitFinally = () => {
    if (!shouldCloseOnFinallyRef.current) {
      return;
    }
    handleClose();
  };

  const handleSubmit = async (profile: ProfileFormSubmitPayload) => {
    if (!profile.name.trim()) {
      toast.error(t('auth.enterName'));
      return;
    }

    if (!currentUser) {
      toast.error(t('auth.accountNotFound'));
      return;
    }

    shouldCloseOnFinallyRef.current = true;
    setIsSubmitting(true);
    let hasError = false;
    try {
      let updatedPicture = profile.picture || currentUser.picture || '';
      let updatedAvatar = currentUser.avatar ?? null;
      let nostrPicture =
        profile.picture || currentUser.avatar?.nostrUri || currentUser.picture || '';

      if (profile.avatarFile) {
        if (!currentUser.npub) {
          toast.error(t('auth.avatarUploadInfoMissing'));
          hasError = true;
        } else {
          try {
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
          } catch (error) {
            hasError = true;
            errorHandler.log('ProfileEditDialog.avatarUploadFailed', error, {
              context: 'ProfileEditDialog.handleSubmit',
            });
          }
        }
      }

      if (currentUser?.npub) {
        try {
          await TauriApi.updatePrivacySettings({
            npub: currentUser.npub,
            publicProfile,
            showOnlineStatus,
          });
        } catch (error) {
          hasError = true;
          errorHandler.log('ProfileEditDialog.privacyUpdateFailed', error, {
            context: 'ProfileEditDialog.handleSubmit',
          });
        }
      }

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
      } catch (error) {
        hasError = true;
        errorHandler.log('ProfileEditDialog.submitFailed', error, {
          context: 'ProfileEditDialog.handleSubmit',
        });
      }

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

      try {
        await syncAvatar({ force: true });
      } catch (error) {
        hasError = true;
        errorHandler.log('ProfileEditDialog.avatarSyncFailed', error, {
          context: 'ProfileEditDialog.handleSubmit',
        });
      } finally {
        useOfflineStore.getState().updateLastSyncedAt();
      }

      if (hasError) {
        toast.error(t('auth.profileUpdateFailed'));
      } else {
        toast.success(t('auth.profileUpdated'));
      }
    } catch (error) {
      toast.error(t('auth.profileUpdateFailed'));
      errorHandler.log('ProfileEditDialog.submitFailed', error, {
        context: 'ProfileEditDialog.handleSubmit',
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl space-y-6">
        <DialogHeader>
          <DialogTitle>{t('auth.editProfile')}</DialogTitle>
          <DialogDescription>{t('auth.editProfileDescription')}</DialogDescription>
        </DialogHeader>
        <ProfileForm
          initialValues={initialValues}
          onSubmit={handleSubmit}
          onCancel={handleClose}
          cancelLabel={t('auth.cancel')}
          submitLabel={isSubmitting ? t('auth.saving') : t('auth.save')}
          isSubmitting={isSubmitting}
          onSubmitFinally={handleSubmitFinally}
        />
      </DialogContent>
    </Dialog>
  );
}
