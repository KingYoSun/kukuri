import type { Meta, StoryObj } from '@storybook/react-vite';

const meta = {
  title: 'Foundations/Typography',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const scale = [
  { token: '--text-display', role: 'Display', note: 'clamp(1.9rem, 4vw, 3.5rem) — トピック見出し / ヒーロー' },
  { token: '--text-h1', role: 'Heading 1', note: '1.5rem / 24px' },
  { token: '--text-h2', role: 'Heading 2', note: '1.25rem / 20px' },
  { token: '--text-h3', role: 'Heading 3', note: '1rem / 16px — カード見出し' },
  { token: '--text-body-reading', role: 'Body Reading', note: '0.9375rem / 15px — post / thread 本文' },
  { token: '--text-body', role: 'Body', note: '0.875rem / 14px — 既定の本文・入力' },
  { token: '--text-caption', role: 'Caption', note: '0.75rem / 12px — メタ情報・補助' },
];

function TypographyPreview() {
  return (
    <div className='min-h-screen bg-[var(--shell-background)] px-6 py-8 text-foreground'>
      <div className='mx-auto flex max-w-[860px] flex-col gap-8 rounded-[var(--radius-panel)] border border-[var(--border-subtle)] bg-[var(--surface-panel)] p-6'>
        <div className='flex flex-col gap-2'>
          <p className='eyebrow'>foundations</p>
          <h1 className='text-3xl font-semibold tracking-[-0.03em]'>Type scale</h1>
          <p className='max-w-[60ch] text-sm text-[var(--muted-foreground)]'>
            shell の font-size は <code>--text-*</code> トークンへ集約している。和文は{' '}
            <code>--font-sans</code> のフォールバックチェーン、ID / ハッシュは <code>--font-mono</code>{' '}
            を使う。
          </p>
        </div>
        <div className='flex flex-col gap-6'>
          {scale.map((item) => (
            <div
              key={item.token}
              className='flex flex-col gap-1 border-b border-[var(--border-subtle)] pb-4'
            >
              <div style={{ fontSize: `var(${item.token})`, lineHeight: 1.2 }}>
                かくり kukuri 0123 — {item.role}
              </div>
              <div className='flex flex-wrap gap-3 text-xs text-[var(--muted-foreground-soft)]'>
                <code>{item.token}</code>
                <span>{item.note}</span>
              </div>
            </div>
          ))}
        </div>
        <div className='flex flex-col gap-2 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-muted)] p-4'>
          <strong className='text-sm'>Monospace（--font-mono）</strong>
          <code className='break-all'>npub1exampleabcdef0123456789 / share:kukuri:topic:demo:channel-1</code>
        </div>
      </div>
    </div>
  );
}

export const Scale: Story = {
  render: () => <TypographyPreview />,
};
