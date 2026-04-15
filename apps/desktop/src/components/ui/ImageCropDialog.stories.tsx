import { useMemo, useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { Button } from '@/components/ui/button';

import { ImageCropDialog } from './ImageCropDialog';

const meta = {
  title: 'UI/ImageCropDialog',
  parameters: {
    layout: 'centered',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

function ImageCropDialogStory() {
  const [open, setOpen] = useState(false);
  const sourceFile = useMemo(
    () =>
      new File(
        [
          new Blob(
            [
              '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><defs><linearGradient id="bg" x1="0%" x2="100%" y1="0%" y2="100%"><stop offset="0%" stop-color="#f59d62"/><stop offset="100%" stop-color="#00b3a4"/></linearGradient></defs><rect width="512" height="512" rx="96" fill="url(#bg)"/><circle cx="180" cy="180" r="64" fill="#f6f1e8"/><path d="M84 388c58-78 124-118 172-118 54 0 106 26 172 118" fill="none" stroke="#101923" stroke-width="40" stroke-linecap="round"/></svg>',
            ],
            { type: 'image/svg+xml' }
          ),
        ],
        'profile-preview.svg',
        { type: 'image/svg+xml' }
      ),
    []
  );

  return (
    <div className='flex flex-col items-center gap-4'>
      <Button type='button' onClick={() => setOpen(true)}>
        Open crop dialog
      </Button>
      <ImageCropDialog
        open={open}
        file={sourceFile}
        title='Crop image'
        description='Adjust the square crop and zoom before saving.'
        confirmLabel='Save crop'
        onOpenChange={setOpen}
        onConfirm={() => setOpen(false)}
      />
    </div>
  );
}

export const Default: Story = {
  render: () => <ImageCropDialogStory />,
};
