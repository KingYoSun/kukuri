import type { Meta, StoryObj } from '@storybook/react-vite';

import { StatusBadge } from '@/components/StatusBadge';

const meta = {
  title: 'UI/StatusBadge',
  component: StatusBadge,
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof StatusBadge>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: 'idle',
  },
  render: () => (
    <div className='flex flex-wrap gap-3 bg-[var(--shell-background)] p-8 text-foreground'>
      <StatusBadge label='idle' />
      <StatusBadge label='relay-assisted' tone='accent' />
      <StatusBadge label='rotation required' tone='warning' />
      <StatusBadge label='error' tone='destructive' />
    </div>
  ),
};
