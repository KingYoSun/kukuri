import type { Meta, StoryObj } from '@storybook/react-vite';

import { Field } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

const meta = {
  title: 'UI/Field',
  component: Field,
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta<typeof Field>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    label: 'Topic',
  },
  render: () => (
    <div className='min-h-screen bg-[var(--shell-background)] p-8 text-foreground'>
      <div className='grid max-w-[760px] gap-5 md:grid-cols-2'>
        <Field label='Topic'>
          <Input defaultValue='kukuri:topic:demo' />
        </Field>
        <Field label='Audience'>
          <Select defaultValue='public' aria-label='Audience'>
            <option value='public'>Public</option>
            <option value='friends'>Friends</option>
            <option value='private'>Private</option>
          </Select>
        </Field>
        <Field
          className='md:col-span-2'
          label='Composer'
          hint='Shared field spacing, label treatment, and error messaging live here.'
          message='Pending validation sample'
        >
          <Textarea defaultValue='Write a post' />
        </Field>
      </div>
    </div>
  ),
};
