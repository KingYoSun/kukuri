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
  { name: 'Panel Muted', value: 'var(--surface-panel-muted)' },
  { name: 'Panel Soft', value: 'var(--surface-panel-soft)' },
  { name: 'Input', value: 'var(--surface-input)' },
  { name: 'Primary Surface', value: 'var(--surface-button-primary)' },
  { name: 'Accent', value: 'var(--accent)' },
  { name: 'Destructive', value: 'var(--destructive)' },
];

const semanticPairs = [
  {
    name: 'Primary action',
    background: 'var(--surface-button-primary)',
    foreground: 'var(--primary-foreground)',
  },
  {
    name: 'Accent state',
    background: 'var(--surface-accent-soft)',
    foreground: 'var(--accent-foreground)',
  },
  {
    name: 'Destructive notice',
    background: 'var(--surface-destructive-soft)',
    foreground: 'var(--destructive)',
  },
];

function TokensPreview({ width }: { width: number }) {
  return (
    <div className='min-h-screen bg-[var(--shell-background)] px-6 py-8 text-foreground'>
      <div
        className='mx-auto flex flex-col gap-8 rounded-[var(--radius-panel)] border border-[var(--border-subtle)] bg-[var(--surface-panel)] p-6'
        style={{ width }}
      >
        <div className='flex flex-col gap-2'>
          <p className='eyebrow'>runtime tokens</p>
          <h1 className='text-3xl font-semibold tracking-[-0.03em]'>Token confirmation surface</h1>
          <p className='max-w-[60ch] text-sm text-[var(--muted-foreground)]'>
            This story renders the values from tokens.css. DESIGN.md is the product contract;
            switch the Storybook theme to inspect the executed dark and light values.
          </p>
        </div>
        <div className='grid gap-4 sm:grid-cols-2 xl:grid-cols-4'>
          {swatches.map((swatch) => (
            <div
              key={swatch.name}
              className='flex flex-col gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] p-4'
            >
              <div
                className='h-20 rounded-[calc(var(--radius-input)-2px)] border border-[var(--border-subtle)]'
                style={{ background: swatch.value }}
              />
              <div className='flex flex-col gap-1'>
                <strong>{swatch.name}</strong>
                <code className='text-xs text-[var(--muted-foreground-soft)]'>{swatch.value}</code>
              </div>
            </div>
          ))}
        </div>
        <div className='grid gap-3 md:grid-cols-3'>
          {semanticPairs.map((pair) => (
            <div
              key={pair.name}
              className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] p-4'
              style={{ background: pair.background, color: pair.foreground }}
            >
              <strong className='block'>{pair.name}</strong>
              <code className='text-xs'>{pair.foreground}</code>
              <span aria-hidden='true'> / </span>
              <code className='text-xs'>{pair.background}</code>
            </div>
          ))}
        </div>
        <div className='grid gap-3 text-sm text-[var(--muted-foreground)] md:grid-cols-3'>
          <div className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] p-4'>
            <strong className='block text-foreground'>Typography</strong>
            <p>Runtime font and type roles are shown in Foundations / Typography.</p>
          </div>
          <div className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] p-4'>
            <strong className='block text-foreground'>Radius</strong>
            <p>Runtime spacing and radius are shown in Foundations / Spacing &amp; Radius.</p>
          </div>
          <div className='rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] p-4'>
            <strong className='block text-foreground'>Theme + focus</strong>
            <p>Use the toolbar to compare themes, locale, width, and reduced motion.</p>
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
