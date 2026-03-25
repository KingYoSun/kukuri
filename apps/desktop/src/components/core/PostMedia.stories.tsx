import type { Meta, StoryObj } from '@storybook/react-vite';

import {
  STORY_IMAGE_MEDIA,
  STORY_VIDEO_PLAYABLE_MEDIA,
  STORY_VIDEO_POSTER_MEDIA,
} from '@/components/storyFixtures';

import { PostMedia } from './PostMedia';

const meta = {
  title: 'Core/PostMedia',
  component: PostMedia,
  render: (args) => (
    <div style={{ width: 'min(32rem, calc(100vw - 2rem))' }}>
      <PostMedia {...args} />
    </div>
  ),
} satisfies Meta<typeof PostMedia>;

export default meta;

type Story = StoryObj<typeof meta>;

export const ImageReady: Story = {
  args: {
    media: STORY_IMAGE_MEDIA,
  },
};

export const VideoPosterOnly: Story = {
  args: {
    media: STORY_VIDEO_POSTER_MEDIA,
  },
};

export const VideoPlayable: Story = {
  args: {
    media: STORY_VIDEO_PLAYABLE_MEDIA,
  },
};
