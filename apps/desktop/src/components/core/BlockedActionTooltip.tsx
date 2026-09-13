import { cloneElement, type ButtonHTMLAttributes, type ReactElement, useId } from 'react';

import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

type BlockedActionTooltipProps = {
  reason: string;
  children: ReactElement<ButtonHTMLAttributes<HTMLButtonElement>>;
};

// #992: ブロック中に使えない関係操作。画面上の文字を増やさず、理由は hover / focus の tooltip と
// accessible description で示す。native disabled にせず focus 可能なままにして keyboard でも理由へ届かせる。
export function BlockedActionTooltip({ reason, children }: BlockedActionTooltipProps) {
  const reasonId = useId();
  return (
    <TooltipProvider delayDuration={180}>
      <Tooltip>
        <TooltipTrigger asChild>
          {cloneElement(children, {
            // pending 用の aria-disabled（ProfileRefreshButton 等）と見た目を分けるため専用 class を付ける。
            className: cn(children.props.className, 'button-blocked-action'),
            'aria-disabled': true,
            'aria-describedby': reasonId,
            onClick: (event) => event.preventDefault(),
          })}
        </TooltipTrigger>
        <span id={reasonId} className='sr-only'>
          {reason}
        </span>
        <TooltipContent>{reason}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
