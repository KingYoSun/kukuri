import { Pin, RefreshCw } from 'lucide-react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { IconButton } from './icon-button';

const meta = {
  title: 'UI/IconButton',
  component: IconButton,
  parameters: {
    layout: 'centered',
  },
} satisfies Meta<typeof IconButton>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: 'Refresh notifications',
    variant: 'ghost',
    children: <RefreshCw className='size-4' aria-hidden='true' />,
  },
};

export const Pressed: Story = {
  args: {
    label: 'Unpin Timeline',
    variant: 'secondary',
    'aria-pressed': true,
    children: <Pin className='size-4' aria-hidden='true' />,
  },
};

export const Disabled: Story = {
  args: {
    label: 'Clear conversation',
    variant: 'ghost',
    disabled: true,
    children: <RefreshCw className='size-4' aria-hidden='true' />,
  },
};
