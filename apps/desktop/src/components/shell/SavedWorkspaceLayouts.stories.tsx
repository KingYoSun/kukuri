import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';
import { userEvent, within } from 'storybook/test';

import { captureSavedWorkspaceLayout } from '@/shell/savedWorkspaceLayouts';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';

import { SavedWorkspaceLayouts } from './SavedWorkspaceLayouts';

type StoryState = 'empty' | 'active' | 'dirty';

function SavedWorkspaceLayoutsStory({ state = 'active' }: { state?: StoryState }) {
  const [store] = useState(() => {
    const nextStore = createDesktopShellStore();
    if (state === 'empty') return nextStore;
    const workspace = nextStore.getState().workspaceState;
    const layout = captureSavedWorkspaceLayout('review-layout', 'Daily workspace', workspace);
    nextStore.setState({
      savedWorkspaceLayouts: [layout],
      workspaceState: {
        ...workspace,
        activeLayoutId: layout.id,
        columns:
          state === 'dirty'
            ? workspace.columns.map((column) => ({ ...column, preferredDesktopSpan: 2 as const }))
            : workspace.columns,
      },
    });
    return nextStore;
  });

  return (
    <DesktopShellStoreContext.Provider value={store}>
      <div className='shell-phase1 min-h-screen bg-[var(--background)] p-6 text-foreground'>
        <div className='mx-auto max-w-2xl rounded-[var(--radius-panel)] bg-[var(--surface-panel-solid)] p-4'>
          <SavedWorkspaceLayouts onActivateLayout={() => {}} />
        </div>
      </div>
    </DesktopShellStoreContext.Provider>
  );
}

const meta = {
  title: 'Shell/SavedWorkspaceLayouts',
  component: SavedWorkspaceLayoutsStory,
  parameters: { layout: 'fullscreen', reviewCanvas: true },
} satisfies Meta<typeof SavedWorkspaceLayoutsStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = { args: { state: 'empty' } };
export const Active: Story = { args: { state: 'active' } };
export const Dirty: Story = { args: { state: 'dirty' } };
export const StorageError: Story = {
  args: { state: 'empty' },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const storagePrototype = Object.getPrototypeOf(window.localStorage) as Storage;
    const originalSetItem = storagePrototype.setItem;
    storagePrototype.setItem = () => {
      throw new DOMException('Quota exceeded', 'QuotaExceededError');
    };
    await userEvent.type(canvas.getByLabelText('Layout name'), 'Unavailable storage');
    await userEvent.click(canvas.getByRole('button', { name: 'Save new' }));
    storagePrototype.setItem = originalSetItem;
  },
};
