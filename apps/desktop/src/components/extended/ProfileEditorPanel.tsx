import type { FormEventHandler } from 'react';

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
  onFieldChange: (field: keyof ProfileEditorFields, value: string) => void;
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
  onFieldChange,
  onBack,
  onSave,
  onReset,
}: ProfileEditorPanelProps) {
  const disabled = status === 'loading' || saving;

  return (
    <Card className='panel-subsection'>
      <CardHeader>
        <div>
          <h3>My Profile</h3>
          <small>{authorLabel}</small>
        </div>
        {onBack ? (
          <Button variant='secondary' type='button' onClick={onBack}>
            プロフィールに戻る
          </Button>
        ) : null}
      </CardHeader>

      {status === 'loading' ? <Notice>Loading profile…</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      <form className='composer composer-compact' onSubmit={onSave} aria-busy={saving}>
        <Label>
          <span>Display Name</span>
          <Input
            value={fields.displayName}
            onChange={(event) => onFieldChange('displayName', event.target.value)}
            placeholder='Visible label'
            disabled={disabled}
          />
        </Label>
        <Label>
          <span>Name</span>
          <Input
            value={fields.name}
            onChange={(event) => onFieldChange('name', event.target.value)}
            placeholder='Canonical name'
            disabled={disabled}
          />
        </Label>
        <Label>
          <span>About</span>
          <Textarea
            value={fields.about}
            onChange={(event) => onFieldChange('about', event.target.value)}
            className='ticket-output'
            placeholder='Short bio'
            disabled={disabled}
          />
        </Label>
        <Label>
          <span>Picture URL</span>
          <Input
            value={fields.picture}
            onChange={(event) => onFieldChange('picture', event.target.value)}
            placeholder='https://...'
            disabled={disabled}
          />
        </Label>

        {status !== 'error' && error ? <p className='error error-inline'>{error}</p> : null}

        <div className='discovery-actions'>
          <Button variant='secondary' type='submit' disabled={!dirty || disabled}>
            Save Profile
          </Button>
          <Button
            variant='secondary'
            type='button'
            disabled={!dirty || disabled}
            onClick={onReset}
          >
            Reset
          </Button>
        </div>
      </form>
    </Card>
  );
}
