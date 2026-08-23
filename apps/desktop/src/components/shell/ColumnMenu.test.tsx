import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { ColumnMenu } from './ColumnMenu';

// WAI-ARIA menu button pattern に沿って、Column menu が keyboard のみで
// 開閉・移動・選択できることを検証する(監査 B4)。

function renderMenu(overrides: Partial<Parameters<typeof ColumnMenu>[0]> = {}) {
  const onMoveLeft = vi.fn();
  const onMoveRight = vi.fn();
  const onSpanChange = vi.fn();
  const onPinnedChange = vi.fn();
  const onClose = vi.fn();
  render(
    <div className='shell-phase1'>
      <ColumnMenu
        title='Stream'
        pinned
        span={2}
        spanOptions={[1, 2]}
        onMoveLeft={onMoveLeft}
        onMoveRight={onMoveRight}
        onSpanChange={onSpanChange}
        onPinnedChange={onPinnedChange}
        onClose={onClose}
        {...overrides}
      />
      <button type='button'>Column body action</button>
    </div>
  );
  return { onMoveLeft, onMoveRight, onSpanChange, onPinnedChange, onClose };
}

const trigger = () => screen.getByRole('button', { name: /Stream menu/ });

describe('ColumnMenu keyboard access', () => {
  it('Enter で開いて先頭の menuitem に focus が移る', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');

    expect(screen.getByRole('menu', { name: 'Stream actions' })).toBeVisible();
    expect(trigger()).toHaveAttribute('aria-expanded', 'true');
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream left' })
    );
  });

  it('Space で開いて先頭の menuitem に focus が移る', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard(' ');

    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream left' })
    );
  });

  it('trigger で ArrowDown すると開いて先頭、ArrowUp すると開いて末尾に focus する', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{ArrowDown}');
    expect(screen.getByRole('menu', { name: 'Stream actions' })).toBeVisible();
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream left' })
    );

    await user.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(document.activeElement).toBe(trigger());

    await user.keyboard('{ArrowUp}');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Close Stream' }));
  });

  it('disabled な先頭項目を飛ばして最初の有効な menuitem に focus する', async () => {
    const user = userEvent.setup();
    renderMenu({ onMoveLeft: undefined });

    trigger().focus();
    await user.keyboard('{Enter}');

    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream right' })
    );
  });

  it('ArrowDown / ArrowUp で循環し Home / End で先頭 / 末尾へ移動する', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    await user.keyboard('{ArrowDown}');
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream right' })
    );

    await user.keyboard('{ArrowUp}{ArrowUp}');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Close Stream' }));

    await user.keyboard('{Home}');
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream left' })
    );

    await user.keyboard('{End}');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Close Stream' }));
  });

  it('menuitem を Enter で選ぶと action 実行後に閉じて trigger へ focus が戻る', async () => {
    const user = userEvent.setup();
    const { onMoveLeft, onMoveRight } = renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    await user.keyboard('{ArrowDown}');
    await user.keyboard('{Enter}');

    expect(onMoveRight).toHaveBeenCalledTimes(1);
    expect(onMoveLeft).not.toHaveBeenCalled();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(trigger()).toHaveAttribute('aria-expanded', 'false');
    expect(document.activeElement).toBe(trigger());
  });

  it('span の menuitemradio を Space で選ぶと onSpanChange が呼ばれる', async () => {
    const user = userEvent.setup();
    const { onSpanChange } = renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    await user.keyboard('{ArrowDown}{ArrowDown}');
    expect(document.activeElement).toBe(screen.getByRole('menuitemradio', { name: '1 span' }));
    await user.keyboard(' ');

    expect(onSpanChange).toHaveBeenCalledWith(1);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(document.activeElement).toBe(trigger());
  });

  it('Escape で閉じて trigger へ focus が戻る', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    await user.keyboard('{Escape}');

    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(document.activeElement).toBe(trigger());
  });

  it('Tab で閉じて trigger へ focus が戻る', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    await user.keyboard('{Tab}');

    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(document.activeElement).toBe(trigger());
  });

  it('focus が menu 外へ移ると閉じる', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    expect(screen.getByRole('menu')).toBeVisible();

    act(() => {
      screen.getByRole('button', { name: 'Column body action' }).focus();
    });

    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('menu 内に focus がある間は Column 本文の scroll で閉じない', async () => {
    const user = userEvent.setup();
    renderMenu();

    trigger().focus();
    await user.keyboard('{Enter}');
    const body = screen.getByRole('button', { name: 'Column body action' });
    body.dispatchEvent(new Event('scroll', { bubbles: true }));

    expect(screen.getByRole('menu')).toBeVisible();
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream left' })
    );
  });

  it('pointer による開閉は維持される', async () => {
    const user = userEvent.setup();
    renderMenu();

    await user.click(trigger());
    expect(screen.getByRole('menu')).toBeVisible();
    await user.click(trigger());
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
});
