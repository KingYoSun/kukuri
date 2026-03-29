import { useState, type FormEvent } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { ProfileEditorPanel } from './ProfileEditorPanel';
import type { ProfileEditorFields } from './types';

const meta = {
  title: 'Extended/ProfileEditorPanel',
  component: ProfileEditorPanel,
} satisfies Meta<typeof ProfileEditorPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

const STORY_ARGS = {
  authorLabel: 'Local Author',
  status: 'ready',
  saving: false,
  dirty: false,
  error: null,
  fields: {
    displayName: 'Local Author',
    name: 'local-author',
    about: 'Maintains shell UI migration work.',
  },
  picturePreviewSrc: 'https://example.com/avatar.png',
  hasPicture: true,
  pictureInputKey: 0,
  onFieldChange: () => undefined,
  onPictureSelect: () => undefined,
  onPictureClear: () => undefined,
  onSave: (event: FormEvent<HTMLFormElement>) => event.preventDefault(),
  onReset: () => undefined,
} satisfies React.ComponentProps<typeof ProfileEditorPanel>;

function ProfileStory({
  status = 'ready',
  error = null,
}: {
  status?: 'loading' | 'ready' | 'error';
  error?: string | null;
}) {
  const [fields, setFields] = useState<ProfileEditorFields>({
    displayName: 'Local Author',
    name: 'local-author',
    about: 'Maintains shell UI migration work.',
  });

  return (
    <ProfileEditorPanel
      authorLabel='Local Author'
      status={status}
      saving={false}
      dirty={false}
      error={error}
      fields={fields}
      picturePreviewSrc='https://example.com/avatar.png'
      hasPicture={true}
      pictureInputKey={0}
      onFieldChange={(field, value) => setFields((current) => ({ ...current, [field]: value }))}
      onPictureSelect={() => undefined}
      onPictureClear={() => undefined}
      onSave={(event) => event.preventDefault()}
      onReset={() => undefined}
    />
  );
}

export const Ready: Story = {
  args: STORY_ARGS,
  render: (args) => <ProfileStory status={args.status} error={args.error} />,
};

export const ErrorState: Story = {
  args: {
    ...STORY_ARGS,
    status: 'error',
    error: 'profile sync failed',
  },
  render: (args) => <ProfileStory status={args.status} error={args.error} />,
};
