import type { Meta, StoryObj } from '@storybook/react-vite';

const meta = {
  title: 'Foundations/Spacing & Radius',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const spacing = [
  { token: '--space-2xs', px: 4 },
  { token: '--space-xs', px: 8 },
  { token: '--space-sm', px: 12 },
  { token: '--space-md', px: 16 },
  { token: '--space-lg', px: 24 },
  { token: '--space-xl', px: 32 },
  { token: '--space-2xl', px: 48 },
];

const radius = [
  { token: '--radius-xs', label: '8px' },
  { token: '--radius-sm', label: '12px' },
  { token: '--radius-input', label: '14px' },
  { token: '--radius', label: '16px' },
  { token: '--radius-panel', label: '22px' },
  { token: '--radius-pill', label: 'pill' },
];

function Preview() {
  return (
    <div className='min-h-screen bg-[var(--shell-background)] px-6 py-8 text-foreground'>
      <div className='mx-auto flex max-w-[860px] flex-col gap-8 rounded-[var(--radius-panel)] border border-[var(--border-subtle)] bg-[var(--surface-panel)] p-6'>
        <div className='flex flex-col gap-2'>
          <p className='eyebrow'>foundations</p>
          <h1 className='text-3xl font-semibold tracking-[-0.03em]'>Spacing &amp; Radius</h1>
          <p className='max-w-[60ch] text-sm text-[var(--muted-foreground)]'>
            gap / padding / margin は <code>--space-*</code>（4px ベース）、角丸は <code>--radius-*</code>{' '}
            に集約している。
          </p>
        </div>
        <div className='flex flex-col gap-3'>
          {spacing.map((item) => (
            <div key={item.token} className='flex items-center gap-4'>
              <code className='w-40 text-xs text-[var(--muted-foreground-soft)]'>{item.token}</code>
              <div
                className='h-4 bg-[var(--surface-button-primary)]'
                style={{ width: `var(${item.token})` }}
              />
              <span className='text-xs text-[var(--muted-foreground)]'>{item.px}px</span>
            </div>
          ))}
        </div>
        <div className='flex flex-wrap gap-4'>
          {radius.map((item) => (
            <div key={item.token} className='flex flex-col items-center gap-2'>
              <div
                className='size-16 border border-[var(--border-subtle)] bg-[var(--surface-panel-muted)]'
                style={{ borderRadius: `var(${item.token})` }}
              />
              <code className='text-xs text-[var(--muted-foreground-soft)]'>{item.token}</code>
              <span className='text-xs text-[var(--muted-foreground)]'>{item.label}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export const Scales: Story = {
  render: () => <Preview />,
};
