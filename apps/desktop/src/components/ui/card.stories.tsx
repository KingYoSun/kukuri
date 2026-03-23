import type { Meta, StoryObj } from '@storybook/react-vite';

import { Card, CardHeader } from '@/components/ui/card';

const meta = {
  title: 'UI/Card',
  component: Card,
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta<typeof Card>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <div className='min-h-screen bg-[var(--shell-background)] p-8 text-foreground'>
      <div className='mx-auto grid max-w-[1080px] gap-6 md:grid-cols-2'>
        <Card>
          <CardHeader>
            <h3>Primary panel</h3>
            <small>desktop width</small>
          </CardHeader>
          <p className='text-sm text-[var(--muted-foreground)]'>
            Product and diagnostics shells share the same panel framing before IA migration.
          </p>
        </Card>
        <Card tone='accent'>
          <CardHeader>
            <h3>Accent panel</h3>
            <small>sync + diagnostics</small>
          </CardHeader>
          <p className='text-sm text-[var(--muted-foreground)]'>
            Accent tone remains reserved for overview surfaces and summary diagnostics.
          </p>
        </Card>
      </div>
    </div>
  ),
};
