import type { Meta, StoryObj } from '@storybook/react-vite';

const meta = {
  title: 'Foundations/Elevation',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const levels = [
  { token: '--shadow-panel', label: 'Panel — カード / パネル' },
  { token: '--shadow-dropdown', label: 'Dropdown — popover / notice / メトリクス' },
  { token: '--shadow-button-primary', label: 'Button Primary — CTA' },
];

function Preview() {
  return (
    <div className='min-h-screen bg-[var(--shell-background)] px-6 py-10 text-foreground'>
      <div className='mx-auto flex max-w-[860px] flex-col gap-10'>
        <div className='flex flex-col gap-2'>
          <p className='eyebrow'>foundations</p>
          <h1 className='text-3xl font-semibold tracking-[-0.03em]'>Elevation</h1>
          <p className='max-w-[60ch] text-sm text-[var(--muted-foreground)]'>
            実装済みの elevation トークン。<code>--shadow-modal</code> / <code>--shadow-overlay</code>{' '}
            は将来定義（consumer 追加時）。
          </p>
        </div>
        <div className='grid gap-10 sm:grid-cols-3'>
          {levels.map((level) => (
            <div key={level.token} className='flex flex-col items-center gap-3'>
              <div
                className='size-28 rounded-[var(--radius-panel)] border border-[var(--border-subtle)] bg-[var(--surface-panel)]'
                style={{ boxShadow: `var(${level.token})` }}
              />
              <code className='text-xs text-[var(--muted-foreground-soft)]'>{level.token}</code>
              <span className='text-center text-xs text-[var(--muted-foreground)]'>{level.label}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export const Levels: Story = {
  render: () => <Preview />,
};
