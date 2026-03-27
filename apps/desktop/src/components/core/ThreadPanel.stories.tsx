import type { Meta, StoryObj } from '@storybook/react-vite';

import {
  createStoryThreadPanelState,
  createStoryThreadPosts,
} from '@/components/storyFixtures';

import { ThreadPanel } from './ThreadPanel';

const threadPanelState = createStoryThreadPanelState();
const threadPosts = createStoryThreadPosts();

const meta = {
  title: 'Core/ThreadPanel',
  component: ThreadPanel,
  args: {
    state: threadPanelState,
    posts: threadPosts,
    onOpenAuthor: () => undefined,
    onOpenThread: () => undefined,
    onReply: () => undefined,
  },
  render: () => (
    <div style={{ width: 'min(44rem, calc(100vw - 2rem))' }}>
      <ThreadPanel
        state={threadPanelState}
        posts={threadPosts}
        onOpenAuthor={() => undefined}
        onOpenThread={() => undefined}
        onReply={() => undefined}
      />
    </div>
  ),
} satisfies Meta<typeof ThreadPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
