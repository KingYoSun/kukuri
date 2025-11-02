import { useMemo, useState } from 'react';

import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { ProfileForm, type ProfileFormValues } from '@/components/auth/ProfileForm';
import { useAuthStore } from '@/stores/authStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

interface ProfileEditDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ProfileEditDialog({ open, onOpenChange }: ProfileEditDialogProps) {
  const { currentUser, updateUser } = useAuthStore();
  const [isSubmitting, setIsSubmitting] = useState(false);

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

  const handleSubmit = async (profile: ProfileFormValues) => {
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
      await updateNostrMetadata({
        name: profile.name,
        display_name: profile.displayName || profile.name,
        about: profile.about,
        picture: profile.picture,
        nip05: profile.nip05,
      });

      updateUser({
        name: profile.name,
        displayName: profile.displayName || profile.name,
        about: profile.about,
        picture: profile.picture,
        nip05: profile.nip05,
      });

      toast.success('プロフィールを更新しました');
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
