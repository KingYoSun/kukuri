import type { Meta, StoryObj } from '@storybook/react-vite';

import { Button } from '@/components/ui/button';

const meta = {
  title: 'UI/Button',
  component: Button,
  args: {
    children: 'Publish',
  },
  render: (args) => (
    <div className='min-h-screen bg-[var(--shell-background)] p-8 text-foreground'>
      <div className='flex flex-wrap gap-4'>
        <Button {...args} variant='primary'>
          Publish
        </Button>
        <Button {...args} variant='secondary'>
          Secondary
        </Button>
        <Button {...args} variant='ghost'>
          Ghost
        </Button>
      </div>
    </div>
  ),
} satisfies Meta<typeof Button>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
