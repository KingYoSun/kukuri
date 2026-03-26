import type { Meta, StoryObj } from '@storybook/react-vite';

import { STORY_ACTIVE_TOPIC } from '@/components/storyFixtures';

import { ShellTopBar } from './ShellTopBar';

const meta = {
  title: 'Shell/ShellTopBar',
  component: ShellTopBar,
  args: {
    activeTopic: STORY_ACTIVE_TOPIC,
  },
} satisfies Meta<typeof ShellTopBar>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
