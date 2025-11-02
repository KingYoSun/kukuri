import { useEffect, useState } from 'react';
import { Upload, User } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';

export interface ProfileFormValues {
  name: string;
  displayName: string;
  about: string;
  picture: string;
  nip05: string;
}

interface ProfileFormProps {
  initialValues: ProfileFormValues;
  onSubmit: (values: ProfileFormValues) => void | Promise<void>;
  onCancel?: () => void;
  cancelLabel?: string;
  onSkip?: () => void;
  skipLabel?: string;
  submitLabel?: string;
  isSubmitting?: boolean;
}

export function ProfileForm({
  initialValues,
  onSubmit,
  onCancel,
  cancelLabel = 'キャンセル',
  onSkip,
  skipLabel = '後で設定',
  submitLabel = '保存',
  isSubmitting = false,
}: ProfileFormProps) {
  const [values, setValues] = useState<ProfileFormValues>(initialValues);

  useEffect(() => {
    setValues(initialValues);
  }, [initialValues]);

  const getInitials = (name: string) => {
    return name
      .split(' ')
      .map((word) => word[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await onSubmit(values);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6" data-testid="profile-form">
      <div className="flex flex-col items-center space-y-2">
        <Avatar className="w-24 h-24">
          <AvatarImage src={values.picture} />
          <AvatarFallback>
            {values.name ? getInitials(values.name) : <User className="w-12 h-12" />}
          </AvatarFallback>
        </Avatar>
        <Button type="button" variant="outline" size="sm">
          <Upload className="w-4 h-4 mr-2" />
          画像をアップロード
        </Button>
        <p className="text-xs text-muted-foreground">またはURLを下に入力</p>
      </div>

      <div className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="name">名前 *</Label>
          <Input
            id="name"
            value={values.name}
            onChange={(event) => setValues({ ...values, name: event.target.value })}
            placeholder="表示名"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="displayName">表示名</Label>
          <Input
            id="displayName"
            value={values.displayName}
            onChange={(event) => setValues({ ...values, displayName: event.target.value })}
            placeholder="@handle（省略可）"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="about">自己紹介</Label>
          <Textarea
            id="about"
            value={values.about}
            onChange={(event) => setValues({ ...values, about: event.target.value })}
            placeholder="あなたについて教えてください"
            rows={3}
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="picture">アバター画像URL</Label>
          <Input
            id="picture"
            type="url"
            value={values.picture}
            onChange={(event) => setValues({ ...values, picture: event.target.value })}
            placeholder="https://example.com/avatar.jpg"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="nip05">NIP-05認証</Label>
          <Input
            id="nip05"
            value={values.nip05}
            onChange={(event) => setValues({ ...values, nip05: event.target.value })}
            placeholder="user@example.com"
          />
          <p className="text-xs text-muted-foreground">
            Nostr認証用のメールアドレス形式の識別子（省略可）
          </p>
        </div>
      </div>

  <div className="flex gap-3 flex-wrap">
        {onCancel && (
          <Button
            type="button"
            variant="outline"
            onClick={onCancel}
            className="flex-1"
            disabled={isSubmitting}
          >
            {cancelLabel}
          </Button>
        )}
        {onSkip && (
          <Button
            type="button"
            variant="outline"
            onClick={onSkip}
            className="flex-1"
            disabled={isSubmitting}
          >
            {skipLabel}
          </Button>
        )}
        <Button type="submit" className="flex-1" disabled={isSubmitting}>
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}
