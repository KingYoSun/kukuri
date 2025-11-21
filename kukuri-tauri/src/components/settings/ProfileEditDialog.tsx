import { useEffect, useMemo, useRef, useState } from 'react';

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
      toast.error('名前を入力してください');
      return;
    }

    if (!currentUser) {
      toast.error('アカウント情報が見つかりません');
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
          toast.error('アバターのアップロードに必要な情報が不足しています');
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
      }

      if (hasError) {
        toast.error('プロフィールの更新に失敗しました');
      } else {
        toast.success('プロフィールを更新しました');
      }
    } catch (error) {
      hasError = true;
      toast.error('プロフィールの更新に失敗しました');
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
          <DialogTitle>プロフィール編集</DialogTitle>
          <DialogDescription>表示情報と自己紹介を更新します</DialogDescription>
        </DialogHeader>
        <ProfileForm
          initialValues={initialValues}
          onSubmit={handleSubmit}
          onCancel={handleClose}
          cancelLabel="キャンセル"
          submitLabel={isSubmitting ? '保存中...' : '保存'}
          isSubmitting={isSubmitting}
          onSubmitFinally={handleSubmitFinally}
        />
      </DialogContent>
    </Dialog>
  );
}
