import type { Meta, StoryObj } from '@storybook/react-vite';

const meta = {
  title: 'Foundations/Tokens',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const swatches = [
  { name: 'Background', value: 'var(--background)' },
  { name: 'Panel', value: 'var(--surface-panel)' },
  { name: 'Panel Accent', value: 'var(--surface-panel-accent)' },
  { name: 'Input', value: 'var(--surface-input)' },
  { name: 'Primary Start', value: 'var(--primary-start)' },
  { name: 'Primary End', value: 'var(--primary-end)' },
  { name: 'Accent', value: 'var(--accent)' },
  { name: 'Destructive', value: 'var(--destructive)' },
];

function TokensPreview({ width }: { width: number }) {
  return (
    <div className='min-h-screen bg-[var(--shell-background)] px-6 py-8 text-foreground'>
      <div
        className='mx-auto flex flex-col gap-8 rounded-[var(--radius-panel)] border border-[var(--border-subtle)] bg-[var(--surface-panel)] p-6'
        style={{ width }}
      >
        <div className='flex flex-col gap-2'>
          <p className='eyebrow'>design tokens</p>
          <h1 className='text-3xl font-semibold tracking-[-0.03em]'>kukuri shell foundation</h1>
          <p className='max-w-[60ch] text-sm text-[var(--muted-foreground)]'>
            Phase 0 keeps the existing shell intact while standardizing color, spacing, radius,
            and primitive styling around shared CSS variables.
          </p>
        </div>
        <div className='grid gap-4 sm:grid-cols-2 xl:grid-cols-4'>
          {swatches.map((swatch) => (
            <div
              key={swatch.name}
              className='flex flex-col gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-black/10 p-4'
            >
              <div
                className='h-20 rounded-[calc(var(--radius-input)-2px)] border border-white/10'
                style={{ background: swatch.value }}
              />
              <div className='flex flex-col gap-1'>
                <strong>{swatch.name}</strong>
                <code className='text-xs text-[var(--muted-foreground-soft)]'>{swatch.value}</code>
              </div>
            </div>
          ))}
        </div>
        <div className='grid gap-3 text-sm text-[var(--muted-foreground)] md:grid-cols-3'>
          <div className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-black/10 p-4'>
            <strong className='block text-foreground'>Typography</strong>
            <p>IBM Plex Sans is the shared shell font, with muted copy and strong headline states.</p>
          </div>
          <div className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-black/10 p-4'>
            <strong className='block text-foreground'>Radius</strong>
            <p>Panels keep a 22px frame radius. Inputs and field surfaces use a 14px inner radius.</p>
          </div>
          <div className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-black/10 p-4'>
            <strong className='block text-foreground'>Motion + focus</strong>
            <p>Reduced decoration, visible focus rings, and deliberate gradients remain the default.</p>
          </div>
        </div>
      </div>
    </div>
  );
}

export const DesktopWidth: Story = {
  render: () => <TokensPreview width={1120} />,
};

export const NarrowWidth: Story = {
  render: () => <TokensPreview width={760} />,
};
