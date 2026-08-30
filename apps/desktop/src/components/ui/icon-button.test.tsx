import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RefreshCw } from 'lucide-react';
import { describe, expect, it, vi } from 'vitest';

import { IconButton, IconButtonTooltip } from './icon-button';
import { TooltipProvider } from './tooltip';

function renderButton(props: Partial<React.ComponentProps<typeof IconButton>> = {}) {
  return render(
    <TooltipProvider delayDuration={0} skipDelayDuration={0}>
      <IconButton label='更新' {...props}>
        <RefreshCw aria-hidden='true' />
      </IconButton>
    </TooltipProvider>
  );
}

describe('IconButton', () => {
  it('同じローカライズ済み操作名をアクセシブルネームとtooltipに使う', async () => {
    const user = userEvent.setup();
    renderButton();

    const button = screen.getByRole('button', { name: '更新' });
    await user.hover(button);

    expect(await screen.findByRole('tooltip')).toHaveTextContent('更新');

    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('tooltip')).not.toBeInTheDocument());

    button.focus();
    expect(await screen.findByRole('tooltip')).toHaveTextContent('更新');

    await user.tab();
    await waitFor(() => expect(screen.queryByRole('tooltip')).not.toBeInTheDocument());
  });

  it('既存のARIA状態、click、ref、disabledをbuttonへ透過する', async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    const ref = { current: null } as React.RefObject<HTMLButtonElement | null>;
    const { rerender } = render(
      <TooltipProvider delayDuration={0}>
        <IconButton label='固定を解除' aria-pressed ref={ref} onClick={onClick}>
          <RefreshCw aria-hidden='true' />
        </IconButton>
      </TooltipProvider>
    );

    const button = screen.getByRole('button', { name: '固定を解除' });
    expect(button).toHaveAttribute('aria-pressed', 'true');
    expect(ref.current).toBe(button);
    await user.click(button);
    expect(onClick).toHaveBeenCalledOnce();

    rerender(
      <TooltipProvider delayDuration={0}>
        <IconButton label='固定を解除' disabled onClick={onClick}>
          <RefreshCw aria-hidden='true' />
        </IconButton>
      </TooltipProvider>
    );
    expect(screen.getByRole('button', { name: '固定を解除' })).toBeDisabled();
  });

  it('独自styleのbuttonにも同じtooltip契約を適用できる', async () => {
    const user = userEvent.setup();
    render(
      <IconButtonTooltip label='次の列へ移動'>
        <button className='custom-icon-control' type='button' aria-label='次の列へ移動'>
          <RefreshCw aria-hidden='true' />
        </button>
      </IconButtonTooltip>
    );

    const button = screen.getByRole('button', { name: '次の列へ移動' });
    expect(button).toHaveClass('custom-icon-control');
    button.focus();
    expect(await screen.findByRole('tooltip')).toHaveTextContent('次の列へ移動');
    await user.keyboard('{Escape}');
  });
});
