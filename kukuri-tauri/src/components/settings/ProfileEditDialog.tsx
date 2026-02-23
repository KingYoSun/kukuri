import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useQueryClient } from '@tanstack/react-query';

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
import { buildProfileSavePayload, collectUniqueSaveErrors } from '@/lib/profile/profileSave';
import { syncProfileQueryCaches } from '@/lib/profile/profileQuerySync';

interface ProfileEditDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ProfileEditDialog({ open, onOpenChange }: ProfileEditDialogProps) {
  const { t } = useTranslation();
  const { currentUser, updateUser } = useAuthStore();
  const queryClient = useQueryClient();
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
    if (!currentUser.npub.trim()) {
      toast.error(t('auth.accountNotFound'));
      return;
    }

    shouldCloseOnFinallyRef.current = true;
    setIsSubmitting(true);
    const saveErrors: string[] = [];
    const pushSaveError = (stepLabel: string, error?: unknown) => {
      const details =
        error instanceof Error && error.message.trim().length > 0
          ? `${stepLabel}: ${error.message}`
          : stepLabel;
      saveErrors.push(details);
    };

    try {
      let updatedPicture = profile.picture || currentUser.picture || '';
      let updatedAvatar = currentUser.avatar ?? null;
      let nostrPicture =
        profile.picture || currentUser.avatar?.nostrUri || currentUser.picture || '';

      if (profile.avatarFile) {
        if (!currentUser.npub) {
          toast.error(t('auth.avatarUploadInfoMissing'));
          pushSaveError(t('auth.profileSaveStepAvatar'));
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
            pushSaveError(t('auth.profileSaveStepAvatar'), error);
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
          pushSaveError(t('auth.profileSaveStepPrivacy'), error);
          errorHandler.log('ProfileEditDialog.privacyUpdateFailed', error, {
            context: 'ProfileEditDialog.handleSubmit',
          });
        }
      }

      const payload = buildProfileSavePayload({
        npub: currentUser.npub,
        name: profile.name,
        displayName: profile.displayName,
        about: profile.about,
        picture: nostrPicture,
        nip05: profile.nip05,
        publicProfile,
        showOnlineStatus,
      });

      try {
        await TauriApi.updateUserProfile(payload.localProfile);
      } catch (error) {
        pushSaveError(t('auth.profileSaveStepLocalProfile'), error);
        errorHandler.log('ProfileEditDialog.localProfileUpdateFailed', error, {
          context: 'ProfileEditDialog.handleSubmit',
        });
      }

      try {
        await updateNostrMetadata(payload.nostrMetadata);
      } catch (error) {
        pushSaveError(t('auth.profileSaveStepNostrMetadata'), error);
        errorHandler.log('ProfileEditDialog.submitFailed', error, {
          context: 'ProfileEditDialog.handleSubmit',
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
        await syncAvatar({ force: true });
      } catch (error) {
        pushSaveError(t('auth.profileSaveStepAvatarSync'), error);
        errorHandler.log('ProfileEditDialog.avatarSyncFailed', error, {
          context: 'ProfileEditDialog.handleSubmit',
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
