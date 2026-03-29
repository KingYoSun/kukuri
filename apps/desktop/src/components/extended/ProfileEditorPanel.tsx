import type { ChangeEventHandler, FormEventHandler } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

import { type ExtendedPanelStatus, type ProfileEditorFields } from './types';

type ProfileEditorPanelProps = {
  authorLabel: string;
  status: ExtendedPanelStatus;
  saving: boolean;
  dirty: boolean;
  error: string | null;
  fields: ProfileEditorFields;
  picturePreviewSrc?: string | null;
  hasPicture: boolean;
  pictureInputKey: number;
  onFieldChange: (field: keyof ProfileEditorFields, value: string) => void;
  onPictureSelect: ChangeEventHandler<HTMLInputElement>;
  onPictureClear: () => void;
  onBack?: () => void;
  onSave: FormEventHandler<HTMLFormElement>;
  onReset: () => void;
};

export function ProfileEditorPanel({
  authorLabel,
  status,
  saving,
  dirty,
  error,
  fields,
  picturePreviewSrc,
  hasPicture,
  pictureInputKey,
  onFieldChange,
  onPictureSelect,
  onPictureClear,
  onBack,
  onSave,
  onReset,
}: ProfileEditorPanelProps) {
  const { t } = useTranslation(['profile', 'common']);
  const disabled = status === 'loading' || saving;

  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <div>
          <h3>{t('editor.title')}</h3>
          <small>{authorLabel}</small>
        </div>
        {onBack ? (
          <Button variant='secondary' type='button' onClick={onBack}>
            {t('editor.back')}
          </Button>
        ) : null}
      </CardHeader>

      {status === 'loading' ? <Notice>{t('editor.loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <form className='composer composer-compact' onSubmit={onSave} aria-busy={saving}>
        <Label>
          <span>{t('editor.displayName')}</span>
          <Input
            value={fields.displayName}
            onChange={(event) => onFieldChange('displayName', event.target.value)}
            placeholder={t('editor.placeholders.displayName')}
            disabled={disabled}
          />
        </Label>
        <Label>
          <span>{t('editor.name')}</span>
          <Input
            value={fields.name}
            onChange={(event) => onFieldChange('name', event.target.value)}
            placeholder={t('editor.placeholders.name')}
            disabled={disabled}
          />
        </Label>
        <Label>
          <span>{t('editor.about')}</span>
          <Textarea
            value={fields.about}
            onChange={(event) => onFieldChange('about', event.target.value)}
            className='ticket-output'
            placeholder={t('editor.placeholders.about')}
            disabled={disabled}
          />
        </Label>
        <div className='profile-editor-picture-panel'>
          <Label>
            <span>{t('editor.picture')}</span>
            <Input
              key={pictureInputKey}
              type='file'
              accept='image/*'
              disabled={disabled}
              onChange={onPictureSelect}
            />
          </Label>
          {picturePreviewSrc ? (
            <div className='profile-editor-picture-preview'>
              <img src={picturePreviewSrc} alt={`${authorLabel} avatar`} className='profile-overview-image' />
            </div>
          ) : null}
          <Button
            variant='secondary'
            type='button'
            disabled={!hasPicture || disabled}
            onClick={onPictureClear}
          >
            {t('common:actions.clear', { ns: 'common' })}
          </Button>
        </div>

        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}

        <div className='discovery-actions'>
          <Button variant='secondary' type='submit' disabled={!dirty || disabled}>
            {t('editor.save')}
          </Button>
          <Button
            variant='secondary'
            type='button'
            disabled={!dirty || disabled}
            onClick={onReset}
          >
            {t('editor.reset')}
          </Button>
        </div>
      </form>
    </Card>
  );
}
