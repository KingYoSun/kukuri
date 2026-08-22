import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { columnDraftKey, type ColumnDraftTarget } from '@/shell/slices/columnDrafts';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';

import { ColumnComposerFooter } from './ColumnComposerFooter';

const target: ColumnDraftTarget = {
  columnId: 'timeline-private',
  action: 'post',
  scope: { topicId: 'topic-a', channelId: 'friends' },
};

function renderFooter(active = true) {
  const store = createDesktopShellStore();
  const onActivate = vi.fn();
  const onSubmit = vi.fn(async () => undefined);
  render(
    <DesktopShellStoreContext.Provider value={store}>
      <ColumnComposerFooter
        active={active}
        destinationLabel='Friends · topic-a'
        locale='en'
        onActivate={onActivate}
        onAttachmentSelection={vi.fn(async () => undefined)}
        onRemoveAttachment={vi.fn()}
        onSubmit={onSubmit}
        target={target}
      />
    </DesktopShellStoreContext.Provider>
  );
  return { store, onActivate, onSubmit };
}

describe('ColumnComposerFooter', () => {
  it('expands in place and writes content only to the addressed Draft key', async () => {
    const user = userEvent.setup();
    const view = renderFooter();

    const action = screen.getByRole('button', { name: /Publish to Friends/ });
    expect(action).toHaveTextContent('Publish');
    await user.click(action);
    expect(view.onActivate).toHaveBeenCalledTimes(1);

    const textarea = screen.getByPlaceholderText('Write a post');
    await user.type(textarea, 'scoped draft');
    expect(view.store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      content: 'scoped draft',
      expanded: true,
    });

    await user.click(screen.getByRole('button', { name: 'Close' }));
    await user.click(screen.getByRole('button', { name: /Publish to Friends/ }));
    expect(screen.getByDisplayValue('scoped draft')).toBeVisible();
  });

  it('uses an icon-sized accessible action while its Column is inactive', () => {
    renderFooter(false);
    const action = screen.getByRole('button', { name: /Publish to Friends/ });
    expect(action).toHaveClass('button-icon', 'size-10');
    expect(action.querySelector('span')).toBeNull();
  });
});
