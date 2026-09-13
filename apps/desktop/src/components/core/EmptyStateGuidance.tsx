import { type ReactNode, useId } from 'react';

import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';
import { cn } from '@/lib/utils';

export type EmptyStateGuidanceItem = { id: string; content: ReactNode };
export type EmptyStateGuidanceAction = { id: string; label: string; onClick: () => void };

type EmptyStateGuidanceProps = {
  title: string;
  /** 始め方の手順。2 件以上なら番号付きで並べる。 */
  steps?: EmptyStateGuidanceItem[];
  /** 保存範囲や到達条件など、手順ではない補足。 */
  notes?: EmptyStateGuidanceItem[];
  /** 既存導線の呼出しだけを行う CTA。外部送信・認証・同意・永続化は起こさない。 */
  actions?: EmptyStateGuidanceAction[];
  testId?: string;
  className?: string;
};

/**
 * 取得成功 0 件の空状態(#994)。見出し(既存の空文言)、実ボタンを chip で示す手順、補足、次の行動を同じ場所に置く。
 * 呼出し側が `status === 'ready'` のときだけ描画する(loading / error と混同しない)。
 */
export function EmptyStateGuidance({
  title,
  steps = [],
  notes = [],
  actions = [],
  testId,
  className,
}: EmptyStateGuidanceProps) {
  const titleId = useId();
  return (
    <Notice
      className={cn('shell-empty-guidance', className)}
      role='status'
      aria-labelledby={titleId}
      data-testid={testId}
    >
      <div className='space-y-2'>
        <p id={titleId} className='font-medium'>
          {title}
        </p>
        {steps.length > 1 ? (
          <ol className='list-decimal space-y-1 pl-5 text-sm'>
            {steps.map((step) => (
              <li key={step.id}>{step.content}</li>
            ))}
          </ol>
        ) : steps.length === 1 ? (
          <p className='text-sm'>{steps[0].content}</p>
        ) : null}
        {notes.map((note) => (
          <p key={note.id} className='text-sm'>
            {note.content}
          </p>
        ))}
        {actions.length > 0 ? (
          <div className='flex flex-wrap gap-2'>
            {actions.map((action) => (
              <Button key={action.id} type='button' variant='secondary' onClick={action.onClick}>
                {action.label}
              </Button>
            ))}
          </div>
        ) : null}
      </div>
    </Notice>
  );
}
