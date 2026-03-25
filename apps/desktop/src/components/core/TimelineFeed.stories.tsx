import type { Meta, StoryObj } from '@storybook/react-vite';

import { STORY_TIMELINE_POSTS } from '@/components/storyFixtures';

import { TimelineFeed } from './TimelineFeed';

const meta = {
  title: 'Core/TimelineFeed',
  component: TimelineFeed,
  args: {
    posts: STORY_TIMELINE_POSTS,
    emptyCopy: 'No posts yet for this topic.',
    onOpenAuthor: () => undefined,
    onOpenThread: () => undefined,
    onReply: () => undefined,
  },
  render: () => (
    <div style={{ width: 'min(44rem, calc(100vw - 2rem))' }}>
      <TimelineFeed
        posts={STORY_TIMELINE_POSTS}
        emptyCopy='No posts yet for this topic.'
        onOpenAuthor={() => undefined}
        onOpenThread={() => undefined}
        onReply={() => undefined}
      />
    </div>
  ),
} satisfies Meta<typeof TimelineFeed>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
