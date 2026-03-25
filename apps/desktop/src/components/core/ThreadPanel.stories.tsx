import type { Meta, StoryObj } from '@storybook/react-vite';

import {
  STORY_THREAD_PANEL_STATE,
  STORY_THREAD_POSTS,
} from '@/components/storyFixtures';

import { ThreadPanel } from './ThreadPanel';

const meta = {
  title: 'Core/ThreadPanel',
  component: ThreadPanel,
  args: {
    state: STORY_THREAD_PANEL_STATE,
    posts: STORY_THREAD_POSTS,
    onClearThread: () => undefined,
    onOpenAuthor: () => undefined,
    onOpenThread: () => undefined,
    onReply: () => undefined,
  },
  render: () => (
    <div style={{ width: 'min(44rem, calc(100vw - 2rem))' }}>
      <ThreadPanel
        state={STORY_THREAD_PANEL_STATE}
        posts={STORY_THREAD_POSTS}
        onClearThread={() => undefined}
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
