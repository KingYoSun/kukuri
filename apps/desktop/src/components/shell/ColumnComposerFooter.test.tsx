import { act, fireEvent, render, screen } from '@testing-library/react';
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
        destinationLabel='Mutuals · topic-a'
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

    const action = screen.getByRole('button', { name: /Post to Mutuals/ });
    expect(action).toHaveTextContent('Post');
    await user.click(action);
    expect(view.onActivate).toHaveBeenCalledTimes(1);

    const textarea = screen.getByPlaceholderText('Write a post');
    await user.type(textarea, 'scoped draft');
    expect(view.store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      content: 'scoped draft',
      expanded: true,
    });

    await user.click(screen.getByRole('button', { name: 'Close' }));
    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    expect(screen.getByDisplayValue('scoped draft')).toBeVisible();
  });

  it('uses an icon-sized accessible action while its Column is inactive', () => {
    renderFooter(false);
    const action = screen.getByRole('button', { name: /Post to Mutuals/ });
    expect(action).toHaveClass('button-icon', 'size-7');
    expect(action.querySelector('span')).toBeNull();
  });

  it('disables button and Ctrl+Enter submission while its Draft is pending', async () => {
    const user = userEvent.setup();
    const view = renderFooter();

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    await user.type(screen.getByPlaceholderText('Write a post'), 'pending draft');
    act(() => {
      const key = columnDraftKey(target);
      view.store.getState().setField('columnDraftsByKey', {
        [key]: {
          ...view.store.getState().columnDraftsByKey[key],
          pending: true,
        },
      });
    });

    expect(screen.getByRole('button', { name: 'Post' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Choose files' })).toBeDisabled();
    expect(screen.getByLabelText(/attachment/i)).toBeDisabled();
    await user.keyboard('{Control>}{Enter}{/Control}');
    expect(view.onSubmit).not.toHaveBeenCalled();
  });
});

// #964: Esc は投稿作成を閉じるだけで、下書き・返信先は store に残し、focus を開始元へ戻す。
describe('ColumnComposerFooter keyboard dismissal (#964)', () => {
  const ALICE = 'a'.repeat(64);
  const mentionCandidates = [
    { pubkey: ALICE, label: 'Alice', displayName: 'Alice', name: 'alice', about: null, picture: null },
  ];

  function renderExpandable() {
    const store = createDesktopShellStore();
    const onSubmit = vi.fn(async () => undefined);
    render(
      <DesktopShellStoreContext.Provider value={store}>
        <ColumnComposerFooter
          active
          destinationLabel='Mutuals · topic-a'
          locale='en'
          mentionCandidates={mentionCandidates}
          onActivate={vi.fn()}
          onAttachmentSelection={vi.fn(async () => undefined)}
          onRemoveAttachment={vi.fn()}
          onSubmit={onSubmit}
          target={target}
        />
      </DesktopShellStoreContext.Provider>
    );
    return { store, onSubmit };
  }

  it('Escape in the textarea collapses the composer, keeps the draft, and refocuses the primary action', async () => {
    const user = userEvent.setup();
    const view = renderExpandable();

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    const textarea = screen.getByPlaceholderText('Write a post');
    await user.type(textarea, 'keep me');
    expect(textarea).toHaveFocus();

    await user.keyboard('{Escape}');

    expect(screen.queryByPlaceholderText('Write a post')).not.toBeInTheDocument();
    const action = screen.getByRole('button', { name: /Post to Mutuals/ });
    expect(action).toHaveFocus();
    expect(view.store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      content: 'keep me',
      expanded: false,
    });
    expect(view.onSubmit).not.toHaveBeenCalled();

    await user.click(action);
    expect(screen.getByDisplayValue('keep me')).toBeVisible();
  });

  it('Escape while mention suggestions are open only closes the suggestions', async () => {
    const user = userEvent.setup();
    renderExpandable();

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    await user.click(screen.getByPlaceholderText('Write a post'));
    await user.keyboard('hi @al');
    expect(screen.getByRole('listbox', { name: 'Mention suggestions' })).toBeInTheDocument();

    await user.keyboard('{Escape}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    expect(screen.getByPlaceholderText('Write a post')).toHaveValue('hi @al');

    await user.keyboard('{Escape}');
    expect(screen.queryByPlaceholderText('Write a post')).not.toBeInTheDocument();
  });

  it('Escape during IME composition leaves the composer open', async () => {
    const user = userEvent.setup();
    renderExpandable();

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    const textarea = screen.getByPlaceholderText('Write a post');
    fireEvent.keyDown(textarea, { key: 'Escape', isComposing: true });

    expect(screen.getByPlaceholderText('Write a post')).toBeInTheDocument();
  });

  it('Escape from a non-editable control inside the composer collapses it and is consumed', async () => {
    const user = userEvent.setup();
    renderExpandable();

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    await user.type(screen.getByPlaceholderText('Write a post'), 'draft');
    const closeButton = screen.getByRole('button', { name: 'Close' });
    closeButton.focus();

    // fireEvent は preventDefault された場合に false を返す。global の Escape cascade は
    // defaultPrevented を見て thread / author pane を閉じないため、ここで消費されることを固定する。
    expect(fireEvent.keyDown(closeButton, { key: 'Escape' })).toBe(false);
    expect(screen.queryByPlaceholderText('Write a post')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Post to Mutuals/ })).toHaveFocus();
  });

  it('shows the keyboard hint and opens the keyboard guidance without touching the draft', async () => {
    const user = userEvent.setup();
    const onOpenKeyboardHelp = vi.fn();
    const store = createDesktopShellStore();
    render(
      <DesktopShellStoreContext.Provider value={store}>
        <ColumnComposerFooter
          active
          destinationLabel='Mutuals · topic-a'
          locale='en'
          onActivate={vi.fn()}
          onOpenKeyboardHelp={onOpenKeyboardHelp}
          onAttachmentSelection={vi.fn(async () => undefined)}
          onRemoveAttachment={vi.fn()}
          onSubmit={vi.fn(async () => undefined)}
          target={target}
        />
      </DesktopShellStoreContext.Provider>
    );

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    await user.type(screen.getByPlaceholderText('Write a post'), 'kept');
    expect(screen.getByText('Esc closes, Ctrl+Enter sends.')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    expect(onOpenKeyboardHelp).toHaveBeenCalledTimes(1);
    expect(screen.getByPlaceholderText('Write a post')).toHaveValue('kept');
    expect(store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      content: 'kept',
      expanded: true,
    });
  });

  it('the Close button also returns focus to the primary action', async () => {
    const user = userEvent.setup();
    renderExpandable();

    await user.click(screen.getByRole('button', { name: /Post to Mutuals/ }));
    await user.click(screen.getByRole('button', { name: 'Close' }));

    expect(screen.getByRole('button', { name: /Post to Mutuals/ })).toHaveFocus();
  });
});
