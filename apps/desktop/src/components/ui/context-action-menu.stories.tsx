import { useState } from 'react';
import type { Meta, StoryObj } from '@storybook/react-vite';

import {
  ContextActionMenu,
  contextActionMenuPositionFromKeyboard,
  contextActionMenuPositionFromPointer,
  type ContextActionMenuPosition,
} from './context-action-menu';

function ContextActionMenuStory() {
  const [position, setPosition] = useState<ContextActionMenuPosition | null>(null);
  return (
    <div className='min-h-screen bg-[var(--shell-background)] p-8 text-foreground'>
      <button
        type='button'
        className='button button-secondary'
        onContextMenu={(event) => setPosition(contextActionMenuPositionFromPointer(event))}
        onKeyDown={(event) => {
          const next = contextActionMenuPositionFromKeyboard(event);
          if (next) setPosition(next);
        }}
      >
        Right-click or press Shift+F10
      </button>
      <ContextActionMenu
        open={position !== null}
        position={position}
        items={[
          { id: 'copy', label: 'Copy author ID', onSelect: () => undefined },
          { id: 'disabled', label: 'Unavailable action', disabled: true, onSelect: () => undefined },
          { id: 'danger', label: 'Danger action', tone: 'danger', onSelect: () => undefined },
        ]}
        onClose={() => setPosition(null)}
      />
    </div>
  );
}

const meta = {
  title: 'UI/ContextActionMenu',
  component: ContextActionMenu,
  render: () => <ContextActionMenuStory />,
  args: {
    open: false,
    position: null,
    items: [],
    onClose: () => undefined,
  },
} satisfies Meta<typeof ContextActionMenu>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PointerAndKeyboard: Story = {};
