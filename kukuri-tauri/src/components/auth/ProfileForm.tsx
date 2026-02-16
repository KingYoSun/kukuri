import { useTranslation } from 'react-i18next';
import type { MutableRefObject } from 'react';
import { useEffect, useRef, useState } from 'react';
import { Upload, User } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import defaultAvatar from '@/assets/profile/default_avatar.png';
import { errorHandler } from '@/lib/errorHandler';
import { MAX_PROFILE_AVATAR_BYTES } from '@/lib/profile/avatar';

export interface ProfileFormValues {
  name: string;
  displayName: string;
  about: string;
  picture: string;
  nip05: string;
}

export interface ProfileFormAvatarFile {
  bytes: Uint8Array;
  format: string;
  sizeBytes: number;
  fileName: string;
}

export interface ProfileFormSubmitPayload extends ProfileFormValues {
  avatarFile?: ProfileFormAvatarFile;
}

interface ProfileFormProps {
  initialValues: ProfileFormValues;
  onSubmit: (values: ProfileFormSubmitPayload) => void | Promise<void>;
  onCancel?: () => void;
  cancelLabel?: string;
  onSkip?: () => void;
  skipLabel?: string;
  submitLabel?: string;
  isSubmitting?: boolean;
  onSubmitFinally?: () => void;
}

const IMAGE_MIME_BY_EXTENSION: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  webp: 'image/webp',
};

const DIALOG_FILTERS = [
  {
    name: 'Images',
    extensions: Object.keys(IMAGE_MIME_BY_EXTENSION),
  },
];

export function ProfileForm({
  initialValues,
  onSubmit,
  onCancel,
  cancelLabel,
  onSkip,
  skipLabel,
  submitLabel,
  isSubmitting = false,
  onSubmitFinally,
}: ProfileFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<ProfileFormValues>(initialValues);
  const [avatarPreview, setAvatarPreview] = useState<string>(initialValues.picture || '');
  const [selectedAvatar, setSelectedAvatar] = useState<ProfileFormAvatarFile | null>(null);
  const [isAvatarLoading, setIsAvatarLoading] = useState(false);
  const objectUrlRef = useRef<string | null>(null);

  useEffect(() => {
    setValues(initialValues);
    setSelectedAvatar(null);
    setAvatarPreview(initialValues.picture || '');
    releaseObjectUrl(objectUrlRef);
  }, [initialValues]);

  useEffect(() => {
    return () => {
      releaseObjectUrl(objectUrlRef);
    };
  }, []);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    try {
      await onSubmit({
        ...values,
        avatarFile: selectedAvatar ?? undefined,
      });
    } finally {
      onSubmitFinally?.();
    }
  };

  const handleAvatarSelect = async () => {
    try {
      setIsAvatarLoading(true);
      const selection = await open({
        multiple: false,
        directory: false,
        filters: DIALOG_FILTERS,
      });

      if (!selection) {
        return;
      }

      const filePath = Array.isArray(selection) ? selection[0] : selection;
      const format = resolveMimeType(filePath);
      if (!format) {
        toast.error(t('auth.unsupportedImageFormat'));
        return;
      }

      const rawBytes = await readFile(filePath);
      const bytes = rawBytes instanceof Uint8Array ? rawBytes : Uint8Array.from(rawBytes);
      applyAvatarSelection(extractFileName(filePath), format, bytes);
    } catch (error) {
      toast.error(t('auth.imageLoadFailed'));
      errorHandler.log('ProfileForm.avatarLoadFailed', error, {
        context: 'ProfileForm.handleAvatarSelect',
      });
    } finally {
      setIsAvatarLoading(false);
    }
  };

  const applyAvatarSelection = (fileName: string, format: string, bytes: Uint8Array) => {
    if (bytes.byteLength === 0) {
      toast.error(t('auth.imageLoadFailed'));
      return false;
    }
    if (bytes.byteLength > MAX_PROFILE_AVATAR_BYTES) {
      toast.error(t('auth.imageTooLarge'));
      return false;
    }

    const objectUrl = createObjectUrl(bytes, format, objectUrlRef);
    setSelectedAvatar({
      bytes,
      format,
      sizeBytes: bytes.byteLength,
      fileName,
    });
    setAvatarPreview(objectUrl);
    setValues((current) => ({
      ...current,
      picture: '',
    }));
    return true;
  };

  const handlePictureChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const url = event.target.value;
    setValues({
      ...values,
      picture: url,
    });
    setSelectedAvatar(null);
    releaseObjectUrl(objectUrlRef);
    setAvatarPreview(url.trim() ? url : '');
  };

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .filter(Boolean)
      .map((word) => word[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  const previewSrc = avatarPreview || defaultAvatar;

  return (
    <form onSubmit={handleSubmit} className="space-y-6" data-testid="profile-form">
      <div className="flex flex-col items-center space-y-2">
        <Avatar className="h-24 w-24">
          <AvatarImage src={previewSrc} />
          <AvatarFallback>
            {values.name ? getInitials(values.name) : <User className="h-12 w-12" />}
          </AvatarFallback>
        </Avatar>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={handleAvatarSelect}
          disabled={isSubmitting || isAvatarLoading}
          data-testid="profile-avatar-upload"
        >
          <Upload className="mr-2 h-4 w-4" />
          {isAvatarLoading ? t('auth.uploadingImage') : t('auth.uploadImage')}
        </Button>
        <p className="text-xs text-muted-foreground">
          {t('auth.imageFormatHint')}
          {selectedAvatar && (
            <span className="ml-1">
              {selectedAvatar.fileName}（{formatFileSize(selectedAvatar.sizeBytes)}）
              {t('auth.selectedImage')}
            </span>
          )}
        </p>
        <p className="text-xs text-muted-foreground">{t('auth.orEnterUrl')}</p>
      </div>

      <div className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="name">{t('auth.nameRequired')}</Label>
          <Input
            id="name"
            value={values.name}
            onChange={(event) => setValues({ ...values, name: event.target.value })}
            placeholder={t('auth.displayNamePlaceholder')}
            data-testid="profile-name"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="displayName">{t('auth.displayName')}</Label>
          <Input
            id="displayName"
            value={values.displayName}
            onChange={(event) => setValues({ ...values, displayName: event.target.value })}
            placeholder={t('auth.displayNameOptional')}
            data-testid="profile-display-name"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="about">{t('auth.bio')}</Label>
          <Textarea
            id="about"
            value={values.about}
            onChange={(event) => setValues({ ...values, about: event.target.value })}
            placeholder={t('auth.aboutPlaceholder')}
            rows={3}
            data-testid="profile-about"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="picture">{t('auth.avatarUrlLabel')}</Label>
          <Input
            id="picture"
            type="url"
            value={values.picture}
            onChange={handlePictureChange}
            placeholder={t('auth.avatarUrlPlaceholder')}
            disabled={isSubmitting}
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="nip05">{t('auth.nip05Label')}</Label>
          <Input
            id="nip05"
            value={values.nip05}
            onChange={(event) => setValues({ ...values, nip05: event.target.value })}
            placeholder={t('auth.nip05Placeholder')}
            disabled={isSubmitting}
          />
          <p className="text-xs text-muted-foreground">{t('auth.nip05Hint')}</p>
        </div>
      </div>

      <div className="flex flex-wrap gap-3">
        {onCancel && (
          <Button
            type="button"
            variant="outline"
            onClick={onCancel}
            className="flex-1"
            disabled={isSubmitting}
            data-testid="profile-cancel"
          >
            {cancelLabel ?? t('auth.cancel')}
          </Button>
        )}
        {onSkip && (
          <Button
            type="button"
            variant="outline"
            onClick={onSkip}
            className="flex-1"
            disabled={isSubmitting}
            data-testid="profile-skip"
          >
            {skipLabel ?? t('auth.setupLater')}
          </Button>
        )}
        <Button
          type="submit"
          className="flex-1"
          disabled={isSubmitting}
          data-testid="profile-submit"
        >
          {submitLabel ?? t('auth.save')}
        </Button>
      </div>
    </form>
  );
}

function resolveMimeType(filePath: string): string | null {
  const ext = extractExtension(filePath);
  if (!ext) {
    return null;
  }
  const mime = IMAGE_MIME_BY_EXTENSION[ext.toLowerCase()];
  return mime ?? null;
}

function extractFileName(filePath: string): string {
  const segments = filePath.split(/\\|\//);
  return segments[segments.length - 1] ?? 'unknown';
}

function extractExtension(filePath: string): string | null {
  const match = /\.([a-zA-Z0-9]+)$/.exec(filePath);
  return match ? match[1] : null;
}

function releaseObjectUrl(ref: MutableRefObject<string | null>) {
  if (ref.current) {
    URL.revokeObjectURL(ref.current);
    ref.current = null;
  }
}

function createObjectUrl(
  bytes: Uint8Array,
  format: string,
  ref: MutableRefObject<string | null>,
): string {
  releaseObjectUrl(ref);
  const blob = new Blob([bytes as unknown as BlobPart], { type: format });
  const url = URL.createObjectURL(blob);
  ref.current = url;
  return url;
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) {
    return '0B';
  }
  const units = ['B', 'KB', 'MB', 'GB'];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, exponent);
  return `${value.toFixed(exponent === 0 ? 0 : 1)}${units[exponent]}`;
}
