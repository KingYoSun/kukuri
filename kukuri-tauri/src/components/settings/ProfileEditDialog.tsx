import { useMemo, useState } from 'react';

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
    onOpenChange(false);
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

    setIsSubmitting(true);
    try {
      let updatedPicture = profile.picture || currentUser.picture || '';
      let updatedAvatar = currentUser.avatar ?? null;
      let nostrPicture =
        profile.picture || currentUser.avatar?.nostrUri || currentUser.picture || '';

      if (profile.avatarFile) {
        if (!currentUser.npub) {
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

      if (currentUser?.npub) {
        await TauriApi.updatePrivacySettings({
          npub: currentUser.npub,
          publicProfile,
          showOnlineStatus,
        });
      }

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

      toast.success('プロフィールを更新しました');
      await syncAvatar({ force: true });
      handleClose();
    } catch (error) {
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
        />
      </DialogContent>
    </Dialog>
  );
}
