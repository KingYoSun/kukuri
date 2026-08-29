import type { Meta, StoryObj } from '@storybook/react-vite';

import { ColumnContextSelect } from './ColumnContextSelect';

const meta = {
  title: 'Shell/ColumnContextSelect',
  component: ColumnContextSelect,
  args: {
    label: 'Timeline topic',
    value: 'kukuri:topic:general',
    title: 'kukuri:topic:general',
    options: [
      { value: 'kukuri:topic:general', label: 'general' },
      {
        value: 'kukuri:topic:long',
        label: 'A deliberately long topic name for narrow Column review',
      },
    ],
    onChange: () => undefined,
  },
  decorators: [
    (Story) => (
      <div className='shell-phase1 w-48 bg-[var(--surface-panel-accent)] p-3'>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof ColumnContextSelect>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NarrowColumn: Story = {};
