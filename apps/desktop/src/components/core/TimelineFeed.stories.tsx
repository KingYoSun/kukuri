import type { Meta, StoryObj } from '@storybook/react-vite';

import { createStoryTimelinePosts } from '@/components/storyFixtures';
import i18n from '@/i18n';

import { TimelineFeed } from './TimelineFeed';

const timelinePosts = createStoryTimelinePosts();

const meta = {
  title: 'Core/TimelineFeed',
  component: TimelineFeed,
  args: {
    posts: timelinePosts,
    emptyCopy: i18n.t('shell:workspace.noPosts'),
    onOpenAuthor: () => undefined,
    onOpenThread: () => undefined,
    onReply: () => undefined,
  },
  render: () => (
    <div style={{ width: 'min(44rem, calc(100vw - 2rem))' }}>
      <TimelineFeed
        posts={timelinePosts}
        emptyCopy={i18n.t('shell:workspace.noPosts')}
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
