import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { ComposerPanel } from './ComposerPanel';
import { type ComposerDraftMediaView } from './types';

const DRAFT_ITEMS: ComposerDraftMediaView[] = [
  {
    id: 'draft-1',
    sourceName: 'launch-banner.png',
    previewUrl:
      'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 320"><rect width="320" height="320" fill="%2300b3a4"/><rect x="40" y="48" width="240" height="224" rx="20" fill="%23101923"/></svg>',
    attachments: [
      {
        key: 'image_original-launch-banner',
        label: 'image_original',
        mime: 'image/png',
        byteSizeLabel: '128 KB',
      },
    ],
  },
];

function ComposerStory({
  initialValue,
  replyMode = false,
  error = null,
}: {
  initialValue: string;
  replyMode?: boolean;
  error?: string | null;
}) {
  const [value, setValue] = useState(initialValue);
  const [items, setItems] = useState(DRAFT_ITEMS);

  return (
    <div className='w-[min(42rem,calc(100vw-2rem))]'>
      <ComposerPanel
        value={value}
        onChange={(event) => setValue(event.target.value)}
        onSubmit={(event) => event.preventDefault()}
        attachmentInputKey={0}
        onAttachmentSelection={() => undefined}
        draftMediaItems={items}
        onRemoveDraftAttachment={(itemId) =>
          setItems((current) => current.filter((item) => item.id !== itemId))
        }
        composerError={error}
        audienceLabel={replyMode ? 'Imported' : 'Public'}
        replyTarget={
          replyMode
            ? {
                content: 'Existing thread reply target',
                audienceLabel: 'Imported',
              }
            : null
        }
        onClearReply={() => undefined}
      />
    </div>
  );
}

const meta = {
  title: 'Core/ComposerPanel',
  parameters: {
    layout: 'centered',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const PublishState: Story = {
  render: () => <ComposerStory initialValue='Shipping the Phase 2 core flow draft tonight.' />,
};

export const ReplyState: Story = {
  render: () => <ComposerStory initialValue='Following up in thread.' replyMode />,
};

export const ErrorState: Story = {
  render: () => (
    <ComposerStory
      initialValue=''
      error='Video poster generation failed. Remove the draft or retry with another file.'
    />
  ),
};
