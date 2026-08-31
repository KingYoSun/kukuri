import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { MentionHoverCard } from './MentionHoverCard';

const PUBKEY = 'a'.repeat(64);

test('mention trigger copies the complete author ID from pointer and keyboard context menus', async () => {
  const user = userEvent.setup();
  const onClick = vi.fn();
  const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: clipboardWriteText },
  });

  render(
    <MentionHoverCard pubkey={PUBKEY} label='Alice'>
      <button type='button' onClick={onClick}>
        @Alice
      </button>
    </MentionHoverCard>
  );

  const trigger = screen.getByRole('button', { name: '@Alice' });
  fireEvent.contextMenu(trigger, { clientX: 32, clientY: 48 });
  await user.click(screen.getByRole('menuitem', { name: 'Copy user ID' }));
  expect(clipboardWriteText).toHaveBeenLastCalledWith(PUBKEY);

  trigger.focus();
  fireEvent.keyDown(trigger, { key: 'F10', shiftKey: true });
  await user.click(screen.getByRole('menuitem', { name: 'Copy user ID' }));
  expect(clipboardWriteText).toHaveBeenCalledTimes(2);

  trigger.focus();
  fireEvent.keyDown(trigger, { key: 'ContextMenu' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Copy user ID' }), { key: 'Escape' });
  expect(trigger).toHaveFocus();

  await user.click(trigger);
  expect(onClick).toHaveBeenCalledOnce();
});
