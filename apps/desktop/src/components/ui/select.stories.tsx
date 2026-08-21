import type { Meta, StoryObj } from '@storybook/react-vite';

import { Select } from './select';

const meta = {
  title: 'UI/Select',
  component: Select,
  args: {
    'aria-label': 'Connection filter',
    defaultValue: 'connected',
  },
  render: (args) => (
    <div className='w-full max-w-80 space-y-2'>
      <label className='block text-sm font-medium' htmlFor='select-story'>
        Connection filter
      </label>
      <Select {...args} id='select-story'>
        <option value='all'>All topics</option>
        <option value='connected'>Connected</option>
        <option value='disconnected'>Disconnected</option>
      </Select>
    </div>
  ),
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Focused: Story = {
  args: {
    autoFocus: true,
  },
};

export const Disabled: Story = {
  args: {
    disabled: true,
  },
};
