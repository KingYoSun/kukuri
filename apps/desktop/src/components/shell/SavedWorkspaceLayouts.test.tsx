import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test } from 'vitest';

import { SavedWorkspaceLayouts } from '@/components/shell/SavedWorkspaceLayouts';
import { applySavedWorkspaceLayout } from '@/shell/savedWorkspaceLayouts';
import {
  createDesktopShellStore,
  DesktopShellStoreContext,
} from '@/shell/store';

beforeEach(() => window.localStorage.clear());

test('saves, updates, activates, renames and deletes named layouts', async () => {
  const user = userEvent.setup();
  const store = createDesktopShellStore();
  render(
    <DesktopShellStoreContext.Provider value={store}>
      <SavedWorkspaceLayouts
        onActivateLayout={(layout) =>
          store.getState().setField('workspaceState', (current) =>
            applySavedWorkspaceLayout(current, layout)
          )
        }
      />
    </DesktopShellStoreContext.Provider>
  );

  await user.type(screen.getByRole('textbox', { name: 'Layout name' }), 'Research');
  await user.click(screen.getByRole('button', { name: 'Save new layout' }));
  expect(screen.getByText('Research')).toBeInTheDocument();
  expect(store.getState().workspaceState.activeLayoutId).not.toBeNull();

  store.getState().setField('workspaceState', (current) => ({
    ...current,
    columns: current.columns.map((column) => ({ ...column, pinned: false })),
  }));
  expect(await screen.findByText('Unsaved changes')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Save changes to Research' }));
  expect(screen.queryByText('Unsaved changes')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Rename Research' }));
  const renameInput = screen.getByRole('textbox', { name: 'Rename layout' });
  await user.clear(renameInput);
  await user.type(renameInput, 'Reading');
  await user.click(screen.getByRole('button', { name: 'Save layout name' }));
  expect(screen.getByText('Reading')).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Delete Reading' }));
  const dialog = screen.getByRole('dialog');
  await user.click(within(dialog).getByRole('button', { name: 'Delete layout' }));
  expect(screen.queryByText('Reading')).not.toBeInTheDocument();
  expect(store.getState().workspaceState.activeLayoutId).toBeNull();
});

test('rejects duplicate names and confirms replacing dirty workspace', async () => {
  const user = userEvent.setup();
  const store = createDesktopShellStore();
  render(
    <DesktopShellStoreContext.Provider value={store}>
      <SavedWorkspaceLayouts
        onActivateLayout={(layout) =>
          store.getState().setField('workspaceState', (current) =>
            applySavedWorkspaceLayout(current, layout)
          )
        }
      />
    </DesktopShellStoreContext.Provider>
  );

  const name = screen.getByRole('textbox', { name: 'Layout name' });
  await user.type(name, 'Research');
  await user.click(screen.getByRole('button', { name: 'Save new layout' }));
  await user.clear(name);
  await user.type(name, ' research ');
  await user.click(screen.getByRole('button', { name: 'Save new layout' }));
  expect(screen.getByText('A layout with this name already exists.')).toBeInTheDocument();

  store.getState().setField('workspaceState', (current) => ({
    ...current,
    columns: current.columns.map((column) => ({ ...column, pinned: false })),
  }));
  await user.click(screen.getByRole('button', { name: 'Research' }));
  expect(screen.getByRole('dialog')).toHaveTextContent(
    'The current layout has unsaved changes. Continue without saving them?'
  );
});
